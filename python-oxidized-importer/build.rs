// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

fn main() {
    pyo3_build_config::use_pyo3_cfgs();
    link_windows_libpython_import_library();
}

/// Links the libpython import library on Windows.
///
/// On Windows, pyo3-ffi links against libpython using `raw-dylib`, which only
/// provides the symbols that pyo3-ffi itself declares. We call some private CPython
/// functions that pyo3-ffi no longer declares, so we need to link the import library
/// explicitly to make those symbols resolvable.
fn link_windows_libpython_import_library() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let interpreter_config = pyo3_build_config::get();

    // Linking is someone else's responsibility (e.g. PyOxidizer emits its own link lines),
    // or libpython is linked statically, in which case pyo3-ffi does not use `raw-dylib`.
    if interpreter_config.suppress_build_script_link_lines() || !interpreter_config.shared() {
        return;
    }

    if let Some(lib_name) = interpreter_config.lib_name() {
        println!("cargo:rustc-link-lib={}", lib_name);
    }
    if let Some(lib_dir) = interpreter_config.lib_dir() {
        println!("cargo:rustc-link-search=native={}", lib_dir);
    }
}
