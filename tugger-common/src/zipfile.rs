// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use {
    anyhow::{anyhow, Result},
    std::{
        io::{Read, Seek, Write},
        path::Path,
    },
};

pub fn extract_zip<R: Read + Seek, P: AsRef<Path>>(reader: R, path: P) -> Result<()> {
    let mut za = zip::ZipArchive::new(reader)?;

    for i in 0..za.len() {
        let mut file = za.by_index(i)?;

        let enclosed_name = file.enclosed_name().ok_or_else(|| {
            anyhow!(
                "unsafe path in zip archive: {}",
                file.name_raw().escape_ascii()
            )
        })?;
        let dest_path = path.as_ref().join(enclosed_name);
        let parent = dest_path.parent().unwrap();

        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }

        let mut b: Vec<u8> = Vec::new();
        file.read_to_end(&mut b)?;
        let mut fh = std::fs::File::create(dest_path)?;
        fh.write_all(&b)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use {super::*, std::io::Cursor, zip::write::SimpleFileOptions};

    fn make_zip(names: &[&str]) -> Result<Cursor<Vec<u8>>> {
        let mut zw = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for name in names {
            zw.start_file(*name, SimpleFileOptions::default())?;
            zw.write_all(b"data")?;
        }
        let mut cursor = zw.finish()?;
        cursor.set_position(0);
        Ok(cursor)
    }

    #[test]
    fn extract_zip_basic() -> Result<()> {
        let td = tempfile::tempdir()?;
        extract_zip(make_zip(&["foo/bar.txt"])?, td.path())?;
        assert_eq!(
            std::fs::read(td.path().join("foo").join("bar.txt"))?,
            b"data"
        );
        Ok(())
    }

    #[test]
    fn extract_zip_rejects_path_traversal() -> Result<()> {
        let td = tempfile::tempdir()?;
        let dest = td.path().join("dest");
        assert!(extract_zip(make_zip(&["../evil.txt"])?, &dest).is_err());
        assert!(!td.path().join("evil.txt").exists());
        Ok(())
    }
}
