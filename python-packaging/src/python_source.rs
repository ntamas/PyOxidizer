// Copyright 2022 Gregory Szorc.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

/*! Utility functions related to Python source code. */

use {anyhow::Result, once_cell::sync::Lazy};

static RE_CODING: Lazy<regex::bytes::Regex> = Lazy::new(|| {
    regex::bytes::Regex::new(r"^[ \t\f]*#.*?coding[:=][ \t]*([-_.a-zA-Z0-9]+)").unwrap()
});

/// Derive the source encoding from Python source code.
pub fn python_source_encoding(source: &[u8]) -> Vec<u8> {
    // Default source encoding is UTF-8. But per PEP 263, the first or second
    // line of source can match a regular expression to define a custom
    // encoding.
    let lines = source.split(|v| v == &b'\n');

    for (i, line) in lines.enumerate() {
        if i > 1 {
            break;
        }

        if let Some(m) = RE_CODING.find(line) {
            return m.as_bytes().to_vec();
        }
    }

    b"utf-8".to_vec()
}

/// Whether Python source code contains a string.
fn source_contains(source: &[u8], needle: &str) -> bool {
    // We can't just look for the bytes of the string because the source file
    // may be in encodings like UTF-16. The source is decoded to Unicode before
    // looking for the code points.
    let encoding = python_source_encoding(source);

    let encoder = match encoding_rs::Encoding::for_label(&encoding) {
        Some(encoder) => encoder,
        None => encoding_rs::UTF_8,
    };

    let (source, ..) = encoder.decode(source);

    source.contains(needle)
}

/// Whether __file__ occurs in Python source code.
pub fn has_dunder_file(source: &[u8]) -> Result<bool> {
    Ok(source_contains(source, "__file__"))
}

/// Whether Python source code contains the patch that delvewheel adds.
///
/// delvewheel bundles the DLLs of a Windows wheel into a `<name>.libs`
/// directory. It adds a function to the `__init__.py` of each top-level
/// package that puts this directory on the DLL search path. The function is
/// named `_delvewheel_patch_<version>`, or `_delvewheel_init_patch_<version>`
/// in older releases. It locates the directory using `__file__`.
pub fn has_delvewheel_patch(source: &[u8]) -> bool {
    source_contains(source, "def _delvewheel_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_delvewheel_patch() {
        assert!(has_delvewheel_patch(
            b"def _delvewheel_patch_1_11_2():\n    import os\n"
        ));
        assert!(has_delvewheel_patch(
            b"\"\"\"\"\"\"  # start delvewheel patch\ndef _delvewheel_init_patch_0_0_22():\n"
        ));
        assert!(!has_delvewheel_patch(b"import os\n"));
        assert!(!has_delvewheel_patch(b"_delvewheel_patch_1_11_2()\n"));
    }
}
