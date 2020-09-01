use super::*;
use ::vfs04::*;
use std::io::Write;

impl<IO: Read + Seek + Send + 'static> ZipReadOnly<IO> {
    fn normalize_file<'s>(&self, orig: &'s str) -> VfsResult<&'s str> {
        if orig.contains("\\") || orig.ends_with("/") {
            return Err(VfsError::InvalidPath { path: orig.into() }); // Invalid path for file
        }
        let path = if orig.starts_with("/") { &orig[1..] } else { orig };
        if path.split('/').any(|c| c == "" || c == "." || c == "..") {
            return Err(VfsError::InvalidPath { path: orig.into() });
        }
        Ok(path)
    }

    fn normalize_path_dir<'s>(&self, orig: &'s str) -> VfsResult<(&'s str, bool)> {
        if orig == "" || orig == "/" {
            Ok(("", true)) // root dir
        } else if orig.ends_with("/") {
            Ok((self.normalize_file(&orig[..orig.len()-1])?, true))
        } else {
            Ok((self.normalize_file(orig)?, false))
        }
    }
}

impl<IO: Read + Seek + Send + 'static> FileSystem for ZipReadOnly<IO> {
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
        if let Some(i) = self.files.get(path) {
            // ZipReadOnly doesn't allow us to access/read multiple ZipFile s at a time.
            // To play nicely with `vfs`, we sadly need to read the whole thing into memory before returning.
            let mut buf = Vec::new();
            self.archive.lock().unwrap().by_index(*i).unwrap().read_to_end(&mut buf)?;
            Ok(Box::new(std::io::Cursor::new(buf)))
        } else if let Some(_) = self.dirs.get(path) {
            Err(VfsError::Other { message: format!("\"{}\" is a directory, not a file", orig) })
        } else {
            Err(VfsError::FileNotFound { path: orig.into() })
        }
    }

    fn metadata(&self, orig: &str) -> VfsResult<VfsMetadata> {
        let (path, dir) = self.normalize_path_dir(orig)?;
        if let Some(i) = self.files.get(path).filter(|_| !dir) {
            Ok(VfsMetadata { file_type: VfsFileType::File, len: self.archive.lock().unwrap().by_index(*i).map(|f| f.size()).unwrap_or(0) })
        } else if let Some(_) = self.dirs.get(path) {
            eprintln!("Yes I found {:?} OK damnit what gives", dir);
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
        (!dir && self.files.contains_key(path)) || self.dirs.contains_key(path.trim_end_matches(is_path_separator))
    }

    // these all involve writing, which zip::read::ZipReadOnly doesn't support
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

    fn is_empty_or_comment(line: &str) -> bool {
        let line = line.trim_start_matches(char::is_whitespace);
        line == "" || line.starts_with("#") || line.starts_with("//") || line.starts_with(";")
    }

    #[test] fn early_vfs_zip() {
        let zip     = ZipReadOnly::new_strict(File::open("test/data/early-vfs-zip.zip").unwrap()).unwrap();
        let files   = std::fs::read_to_string("test/data/early-vfs-zip.files.txt").unwrap();
        let dirs    = std::fs::read_to_string("test/data/early-vfs-zip.dirs.txt").unwrap();
        let files   = files.split(|ch| "\r\n".contains(ch)).filter(|l| !is_empty_or_comment(l));
        let dirs    = dirs .split(|ch| "\r\n".contains(ch)).filter(|l| !is_empty_or_comment(l));

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
