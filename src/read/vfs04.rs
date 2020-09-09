use crate::{error, ZipReadOnly};
use vfs04::*;
use read_write_at::ReadAt;
use std::convert::*;
use std::io::{self, Read, Write};

impl<IO: ReadAt> ZipReadOnly<IO> {
    fn normalize_file<'s>(&self, orig: &'s str) -> VfsResult<&'s str> {
        if orig.contains('\\') || orig.ends_with('/') {
            return Err(VfsError::InvalidPath { path: orig.into() }); // Invalid path for file
        }
        let path = if orig.starts_with('/') { &orig[1..] } else { orig };
        if path.split('/').any(|c| c == "" || c == "." || c == "..") {
            return Err(VfsError::InvalidPath { path: orig.into() });
        }
        Ok(path)
    }

    fn normalize_path_dir<'s>(&self, orig: &'s str) -> VfsResult<(&'s str, bool)> {
        if orig == "" || orig == "/" {
            Ok(("", true)) // root dir
        } else if orig.ends_with('/') {
            Ok((self.normalize_file(&orig[..orig.len()-1])?, true))
        } else {
            Ok((self.normalize_file(orig)?, false))
        }
    }
}

impl<IO: ReadAt + Send + Sync + 'static> FileSystem for ZipReadOnly<IO> {
    fn read_dir(&self, orig: &str) -> VfsResult<Box<dyn Iterator<Item = String>>> {
        let path = self.normalize_path_dir(orig)?.0;
        if let Some(dir) = self.dirs.get(path) {
            Ok(Box::new(dir.iter().cloned().collect::<Vec<_>>().into_iter())) // Eww
        } else if let Some(_file) = self.files.get(path) {
            Err(VfsError::Other { message: format!("\"{}\" is a file, not a directory", orig) })
        } else {
            Err(VfsError::FileNotFound { path: orig.into() })
        }
    }

    fn open_file(&self, orig: &str) -> VfsResult<Box<dyn SeekAndRead>> {
        let path = self.normalize_file(orig)?;
        if let Some(e) = self.files.get(path) {
            use io::ErrorKind::InvalidData;

            // Header + Compressed blob
            let hacn = e.compressed.checked_add(e.header_size).and_then(|n| n.try_into().ok()).ok_or_else(||
                VfsError::IoError(io::Error::new(InvalidData, "vfs-zip must read compressed file entry into memory, but it is too large"))
            )?;

            let mut hac = Vec::new();
            hac.resize(hacn, 0);
            self.io.read_exact_at(&mut hac[..], e.header_offset)?;
            let mut hac = std::io::Cursor::new(hac);

            // Uncompressed blob
            let uncn = e.uncompressed.try_into().map_err(|_|
                VfsError::IoError(io::Error::new(InvalidData, "vfs-zip must read decompressed file entry into memory, but it is too large"))
            )?;
            let mut unc = Vec::new();
            unc.resize(uncn, 0);
            let mut zf = zip::read::read_zipfile_from_stream(&mut hac).map_err(|e| error::zip2vfs(&path, e))?.ok_or_else(||
                VfsError::IoError(io::Error::new(InvalidData, "expected a file entry, did file contents change underneath this reader?!?"))
            )?;
            zf.read_exact(&mut unc[..])?;

            Ok(Box::new(std::io::Cursor::new(unc)))
        } else if let Some(_) = self.dirs.get(path) {
            Err(VfsError::Other { message: format!("\"{}\" is a directory, not a file", orig) })
        } else {
            Err(VfsError::FileNotFound { path: orig.into() })
        }
    }

    fn metadata(&self, orig: &str) -> VfsResult<VfsMetadata> {
        let (path, dir) = self.normalize_path_dir(orig)?;
        if let Some(e) = self.files.get(path).filter(|_| !dir) {
            Ok(VfsMetadata { file_type: VfsFileType::File, len: e.uncompressed })
        } else if let Some(_) = self.dirs.get(path) {
            Ok(VfsMetadata { file_type: VfsFileType::Directory, len: 0 })
        } else {
            Err(VfsError::FileNotFound { path: orig.into() })
        }
    }

    fn exists(&self, path: &str) -> bool {
        let (path, dir) = match self.normalize_path_dir(path) {
            Ok(pd)  => pd,
            Err(_)  => return false, // XXX
        };
        (!dir && self.files.contains_key(path)) || self.dirs.contains_key(path.trim_end_matches('/'))
    }

    // these all involve writing, which zip::read::ZipArchive doesn't support
    fn create_dir   (&self, _path: &str)            -> VfsResult<()>                { Err(VfsError::NotSupported) }
    fn create_file  (&self, _path: &str)            -> VfsResult<Box<dyn Write>>    { Err(VfsError::NotSupported) }
    fn append_file  (&self, _path: &str)            -> VfsResult<Box<dyn Write>>    { Err(VfsError::NotSupported) }
    fn remove_file  (&self, _path: &str)            -> VfsResult<()>                { Err(VfsError::NotSupported) }
    fn remove_dir   (&self, _path: &str)            -> VfsResult<()>                { Err(VfsError::NotSupported) }
    fn copy_file    (&self, _src: &str, _dst: &str) -> VfsResult<()>                { Err(VfsError::NotSupported) }
    fn move_file    (&self, _src: &str, _dst: &str) -> VfsResult<()>                { Err(VfsError::NotSupported) }
    fn move_dir     (&self, _src: &str, _dst: &str) -> VfsResult<()>                { Err(VfsError::NotSupported) }
}

#[cfg(test)] mod tests {
    use super::*;
    use std::fs::File;
    use std::sync::{Arc, Mutex};

    fn is_empty_or_comment(line: &str) -> bool {
        let line = line.trim_start_matches(char::is_whitespace);
        line == "" || line.starts_with("#") || line.starts_with("//") || line.starts_with(";")
    }

    #[test] fn early_vfs_zip() {
        let files   = std::fs::read_to_string("test/data/early-vfs-zip.files.txt").unwrap();
        let dirs    = std::fs::read_to_string("test/data/early-vfs-zip.dirs.txt").unwrap();
        let files   = files.split(|ch| "\r\n".contains(ch)).filter(|l| !is_empty_or_comment(l)).collect::<Vec<_>>();
        let dirs    = dirs .split(|ch| "\r\n".contains(ch)).filter(|l| !is_empty_or_comment(l)).collect::<Vec<_>>();

        with_zip("File",            files.iter().cloned(), dirs.iter().cloned(), &ZipReadOnly::new_strict(File::open("test/data/early-vfs-zip.zip").unwrap()).unwrap());
        with_zip("Mutex<File>",     files.iter().cloned(), dirs.iter().cloned(), &ZipReadOnly::new_strict(Mutex::new(File::open("test/data/early-vfs-zip.zip").unwrap())).unwrap());
        with_zip("Vec<u8>",         files.iter().cloned(), dirs.iter().cloned(), &ZipReadOnly::new_strict(std::fs::read("test/data/early-vfs-zip.zip").unwrap()).unwrap());
        with_zip("Arc<[u8]>",       files.iter().cloned(), dirs.iter().cloned(), &ZipReadOnly::new_strict(Arc::<[u8]>::from(std::fs::read("test/data/early-vfs-zip.zip").unwrap())).unwrap());
        with_zip("Box<[u8]>",       files.iter().cloned(), dirs.iter().cloned(), &ZipReadOnly::new_strict(Box::<[u8]>::from(std::fs::read("test/data/early-vfs-zip.zip").unwrap())).unwrap());
        with_zip("&'static [u8]",   files.iter().cloned(), dirs.iter().cloned(), &ZipReadOnly::new_strict(Box::leak(Box::<[u8]>::from(std::fs::read("test/data/early-vfs-zip.zip").unwrap()))).unwrap());
        // XXX: vfs04::FileSystem demands 'static which outlives a &[u8] slice
        let _ = ZipReadOnly::new_strict(&std::fs::read("test/data/early-vfs-zip.zip").unwrap()[..]).unwrap();
    }

    fn with_zip<'a>(src: &str, files: impl Iterator<Item = &'a str>, dirs: impl Iterator<Item = &'a str>, zip: &impl FileSystem) {
        eprintln!("{}", src);
        eprintln!("{:=<1$}", "", src.len());
        for file in files {
            for good in &[
                format!("{}", file),
                format!("/{}", file),
            ] {
                eprintln!("{}", good);
                zip.read_dir(&good).err().unwrap();
                zip.open_file(&good).unwrap();
                zip.metadata(&good).unwrap();
                assert_eq!(zip.exists(&good), true);
            }

            for bad in &[
                format!("//{}", file),
                format!("\\{}", file),
                format!("nonexistant/{}", file),
                format!("{}.nonexistant", file),
                format!("{}/", file),
                format!("/{}/", file),
                format!("./{}/", file),
            ] {
                eprintln!("{}", bad);
                zip.read_dir(&bad).err().unwrap();
                zip.open_file(&bad).err().unwrap();
                zip.metadata(&bad).err().unwrap();
                assert_eq!(zip.exists(&bad), false);
            }
        }

        for dir in dirs {
            for good in &[
                format!("{}", dir),
                format!("/{}", dir),
                format!("{}/", dir),
                format!("/{}/", dir),
            ] {
                eprintln!("{}", good);
                let _ = zip.read_dir(&good).unwrap().collect::<Vec<String>>();
                zip.open_file(&good).err().unwrap();
                zip.metadata(&good).unwrap();
                assert_eq!(zip.exists(&good), true);
            }

            for bad in &[
                format!("nonexistant/{}", dir),
                format!("{}.nonexistant", dir),
                format!("/{}/nonexistant", dir),
                format!("./{}/", dir),
                format!("//{}", dir),
                format!("{}//", dir),
                format!("{}\\", dir),
                format!("\\{}", dir),
            ] {
                eprintln!("{}", bad);
                zip.read_dir(&bad).err().unwrap();
                zip.open_file(&bad).err().unwrap();
                zip.metadata(&bad).err().unwrap();
                assert_eq!(zip.exists(&bad), false);
            }
        }
    }
}

