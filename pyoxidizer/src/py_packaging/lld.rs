// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/*!
Linking LLVM bitcode libpython with the Rust toolchain's LLD.

`pgo+lto` python-build-standalone distributions ship libpython as LLVM
bitcode. rustc parses that bitcode when bundling libpython into an rlib, so
its LLVM must be at least as new as the LLVM that produced the bitcode. And
the final link needs an LTO-capable linker built against an equally new LLVM.
The system linkers (GNU ld, Apple ld) can't consume this bitcode in general,
so we link with the `rust-lld` shipped with the Rust toolchain.

Linker arguments are not inherited from dependencies' build scripts, so the
arguments needed to select LLD must be emitted by the build script of the
crate producing the final binary (or passed via `RUSTFLAGS`). To make this
possible from pre-built artifacts, we write a small wrapper around LLD next
to the libpython artifacts and point the C compiler driver at it via `-B`.
*/

use {
    crate::{
        environment::{Environment, RustEnvironment},
        py_packaging::{distribution::AppleSdkInfo, embedding::LinkingAnnotation},
    },
    anyhow::{anyhow, Context, Result},
    apple_sdk::{AppleSdk, ParsedSdk},
    duct::cmd,
    log::warn,
    std::path::{Path, PathBuf},
};

/// Name of the directory holding the LLD wrapper, relative to the artifacts directory.
pub const LLD_WRAPPER_DIR: &str = "lld";

/// Validate that a Rust toolchain can link an LLVM bitcode libpython and
/// resolve the directory holding the toolchain's LLD linker wrappers.
///
/// Returns the Rust sysroot and the directory holding the LLD wrappers.
pub fn resolve_lld_for_llvm_bitcode(
    rust_environment: &RustEnvironment,
    bitcode_version: &semver::Version,
) -> Result<(PathBuf, PathBuf)> {
    if let Some(llvm) = &rust_environment.rust_version.llvm_version {
        if llvm.major < bitcode_version.major {
            return Err(anyhow!(
                "the Python distribution's libpython consists of LLVM bitcode produced by LLVM {}, \
                 but the Rust toolchain ({}) uses LLVM {}.{}, which cannot read it; \
                 use a Rust toolchain built against LLVM {} or newer",
                bitcode_version,
                rust_environment.rustc_exe.display(),
                llvm.major,
                llvm.minor,
                bitcode_version.major
            ));
        }
    } else {
        warn!(
            "unable to determine the LLVM version of the Rust toolchain; linking LLVM bitcode may fail"
        );
    }

    let sysroot = cmd(&rust_environment.rustc_exe, ["--print", "sysroot"])
        .read()
        .context("resolving Rust sysroot")?;

    let sysroot = PathBuf::from(sysroot.trim());

    let lld_dir = sysroot
        .join("lib")
        .join("rustlib")
        .join(&rust_environment.rust_version.host)
        .join("bin")
        .join("gcc-ld");

    if !lld_dir.is_dir() {
        return Err(anyhow!(
            "LLD not found at {}; the Rust toolchain must provide rust-lld to link an LLVM bitcode libpython",
            lld_dir.display()
        ));
    }

    Ok((sysroot, lld_dir))
}

/// A wrapper script that makes a C compiler driver link with the Rust toolchain's LLD.
///
/// The wrapper is written to a directory next to the libpython artifacts and
/// that directory is passed to the compiler driver via `-B`, which makes it
/// look for the linker there first.
///
/// On Linux, the wrapper is named `ld` so it is picked up by any GCC or Clang,
/// even versions too old to understand `-fuse-ld=lld`. On macOS, the wrapper
/// is named `ld64.lld` and `-fuse-ld=lld` is passed in addition; the wrapper
/// also points the dynamic linker at the toolchain's `lib` directory because
/// `rust-lld` in some toolchains (e.g. nightlies around Rust 1.98) lacks the
/// rpath needed to find `libLLVM.dylib`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LldWrapper {
    /// File name of the wrapper script.
    pub file_name: String,

    /// Content of the wrapper script.
    pub script: String,

    /// Whether `-fuse-ld=lld` needs to be passed to the compiler driver.
    pub use_fuse_ld: bool,
}

impl LldWrapper {
    /// Resolve the wrapper for linking a libpython consisting of LLVM bitcode.
    ///
    /// This ensures a Rust toolchain is available and validates that it can
    /// consume the bitcode.
    pub fn new(
        env: &Environment,
        host_triple: &str,
        target_triple: &str,
        bitcode_version: &semver::Version,
    ) -> Result<Self> {
        if target_triple.contains("-windows-") {
            return Err(anyhow!(
                "linking an LLVM bitcode libpython is not supported on Windows"
            ));
        }

        let rust_environment = env
            .ensure_rust_toolchain(Some(target_triple))
            .context("ensuring Rust toolchain available")?;

        let (sysroot, lld_dir) = resolve_lld_for_llvm_bitcode(&rust_environment, bitcode_version)
            .context("resolving linker for LLVM bitcode libpython")?;

        Ok(Self::from_toolchain(
            &sysroot,
            &lld_dir,
            host_triple,
            target_triple,
        ))
    }

    /// Construct a wrapper for the LLD in the given Rust sysroot.
    pub fn from_toolchain(
        sysroot: &Path,
        lld_dir: &Path,
        host_triple: &str,
        target_triple: &str,
    ) -> Self {
        let apple = target_triple.contains("-apple-");

        let (file_name, lld) = if apple {
            ("ld64.lld", lld_dir.join("ld64.lld"))
        } else {
            ("ld", lld_dir.join("ld.lld"))
        };

        let mut script = String::from(
            "#!/bin/sh\n\
             # Generated by PyOxidizer. Links with the Rust toolchain's LLD, which\n\
             # is needed to link the LLVM bitcode in libpython.\n",
        );

        if host_triple.contains("-apple-darwin") {
            script.push_str(&format!(
                "# The toolchain's lib directory is added to the dynamic library search\n\
                 # path so LLD can find libLLVM.dylib.\n\
                 DYLD_LIBRARY_PATH=\"{lib}${{DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}}\"\n\
                 export DYLD_LIBRARY_PATH\n",
                lib = sysroot.join("lib").display(),
            ));
        }

        script.push_str(&format!("exec \"{}\" \"$@\"\n", lld.display()));

        Self {
            file_name: file_name.to_string(),
            script,
            use_fuse_ld: apple,
        }
    }

    /// Directory holding the wrapper, given the artifacts directory.
    pub fn dir(&self, dest_dir: impl AsRef<Path>) -> PathBuf {
        dest_dir.as_ref().join(LLD_WRAPPER_DIR)
    }

    /// Path to the wrapper script, given the artifacts directory.
    pub fn path(&self, dest_dir: impl AsRef<Path>) -> PathBuf {
        self.dir(dest_dir).join(&self.file_name)
    }

    /// Write the wrapper script to the artifacts directory.
    pub fn write(&self, dest_dir: impl AsRef<Path>) -> Result<()> {
        let dir = self.dir(&dest_dir);
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;

        let path = self.path(&dest_dir);
        std::fs::write(&path, self.script.as_bytes())
            .with_context(|| format!("writing {}", path.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                .with_context(|| format!("setting permissions on {}", path.display()))?;
        }

        Ok(())
    }

    /// Arguments to pass to the C compiler driver to link with the wrapper.
    pub fn linker_args(&self, dest_dir: impl AsRef<Path>) -> Vec<String> {
        let mut args = vec![];

        if self.use_fuse_ld {
            args.push("-fuse-ld=lld".to_string());
        }

        args.push(format!("-B{}", self.dir(dest_dir).display()));

        args
    }

    /// Linking annotations to link with the wrapper.
    pub fn linking_annotations(&self, dest_dir: impl AsRef<Path>) -> Vec<LinkingAnnotation> {
        self.linker_args(dest_dir)
            .into_iter()
            .map(LinkingAnnotation::Argument)
            .collect()
    }
}

/// Whether LLD (`ld64.lld`) can link against the given Apple SDK.
///
/// LLD's parser for the SDK's `.tbd` library stubs can lag behind Apple's
/// SDKs. For example, LLD 23 can't parse the stubs in the macOS 27 SDK. We
/// detect this by performing a trivial link against the SDK.
pub fn apple_sdk_usable_by_lld(
    lld_path: &Path,
    sdk: &ParsedSdk,
    sdk_info: &AppleSdkInfo,
    target_triple: &str,
) -> Result<bool> {
    let arch = if target_triple.starts_with("aarch64-") {
        "arm64"
    } else if target_triple.starts_with("x86_64-") {
        "x86_64"
    } else {
        return Ok(true);
    };

    let platform = match sdk_info.platform.as_str() {
        "macosx" => "macos",
        _ => return Ok(true),
    };

    let sdk_version = sdk
        .version()
        .map(|v| v.to_string())
        .unwrap_or_else(|| sdk_info.version.clone());

    let temp_dir = tempfile::Builder::new()
        .prefix("pyoxidizer-lld-probe")
        .tempdir()
        .context("creating temporary directory")?;
    let output_path = temp_dir.path().join("probe.dylib");

    let output = std::process::Command::new(lld_path)
        .arg("-dylib")
        .arg("-arch")
        .arg(arch)
        .arg("-platform_version")
        .arg(platform)
        .arg(&sdk_info.deployment_target)
        .arg(&sdk_version)
        .arg("-syslibroot")
        .arg(sdk.path())
        .arg("-lSystem")
        .arg("-o")
        .arg(&output_path)
        .output()
        .context("invoking ld64.lld to probe Apple SDK compatibility")?;

    if output.status.success() {
        Ok(true)
    } else {
        warn!(
            "LLD cannot link against Apple SDK {}:\n{}",
            sdk.path().display(),
            String::from_utf8_lossy(&output.stderr).trim_end()
        );
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrapper_linux() {
        let w = LldWrapper::from_toolchain(
            Path::new("/sysroot"),
            Path::new("/sysroot/lib/rustlib/x86_64-unknown-linux-gnu/bin/gcc-ld"),
            "x86_64-unknown-linux-gnu",
            "x86_64-unknown-linux-gnu",
        );

        assert_eq!(w.file_name, "ld");
        assert!(!w.use_fuse_ld);
        assert!(w.script.starts_with("#!/bin/sh\n"));
        assert!(w.script.ends_with(
            "exec \"/sysroot/lib/rustlib/x86_64-unknown-linux-gnu/bin/gcc-ld/ld.lld\" \"$@\"\n"
        ));
        assert!(!w.script.contains("DYLD_LIBRARY_PATH"));

        assert_eq!(w.path("/artifacts"), PathBuf::from("/artifacts/lld/ld"));
        assert_eq!(
            w.linker_args("/artifacts"),
            vec!["-B/artifacts/lld".to_string()]
        );
        assert_eq!(
            w.linking_annotations("/artifacts"),
            vec![LinkingAnnotation::Argument("-B/artifacts/lld".to_string())]
        );
    }

    #[test]
    fn test_wrapper_macos() {
        let w = LldWrapper::from_toolchain(
            Path::new("/sysroot"),
            Path::new("/sysroot/lib/rustlib/aarch64-apple-darwin/bin/gcc-ld"),
            "aarch64-apple-darwin",
            "x86_64-apple-darwin",
        );

        assert_eq!(w.file_name, "ld64.lld");
        assert!(w.use_fuse_ld);
        assert!(w.script.contains(
            "DYLD_LIBRARY_PATH=\"/sysroot/lib${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}\"\n"
        ));
        assert!(w.script.ends_with(
            "exec \"/sysroot/lib/rustlib/aarch64-apple-darwin/bin/gcc-ld/ld64.lld\" \"$@\"\n"
        ));

        assert_eq!(
            w.linker_args("/artifacts"),
            vec!["-fuse-ld=lld".to_string(), "-B/artifacts/lld".to_string()]
        );
    }

    #[test]
    fn test_wrapper_write() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;

        let w = LldWrapper::from_toolchain(
            Path::new("/sysroot"),
            Path::new("/sysroot/gcc-ld"),
            "x86_64-unknown-linux-gnu",
            "aarch64-unknown-linux-gnu",
        );
        w.write(temp_dir.path())?;

        let path = temp_dir.path().join("lld").join("ld");
        assert_eq!(std::fs::read_to_string(&path)?, w.script);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path)?.permissions().mode() & 0o777,
                0o755
            );
        }

        Ok(())
    }
}
