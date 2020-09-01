use super::*;

use ::vfs04::*;

use ::zip::result::ZipError;
use ::zip::write::FileOptions;

use std::io::{self, Cursor};
use std::mem::replace;
use std::ops::Drop;
use std::sync::{Arc, Weak};



impl<IO: Write + Seek + Send + 'static> ZipWriteOnly<IO> {
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

impl<IO: Write + Seek + Send + 'static> FileSystem for ZipWriteOnly<IO> {
    fn create_dir(&self, path: &str) -> VfsResult<()> {
        let path = self.normalize_path_dir(path)?.0;
        let mut imp = self.imp.lock().unwrap();
        if path == "" {
            imp.dirs.insert(path.into());
            return Ok(());
        }
        match imp.writer.add_directory(path, FileOptions::default()) {
            Err(ZipError::FileNotFound)             => return Err(VfsError::FileNotFound { path: path.into() }),
            Err(ZipError::Io(e))                    => return Err(VfsError::IoError(e)),
            Err(ZipError::InvalidArchive(e))        => return Err(VfsError::IoError(io::Error::new(io::ErrorKind::InvalidData, e))),
            Err(ZipError::UnsupportedArchive(e))    => return Err(VfsError::Other { message: e.into() }),
            Ok(())                                  => {},
        }
        imp.dirs.insert(path.into());
        Ok(())
    }

    fn create_file(&self, path: &str) -> VfsResult<Box<dyn Write>> {
        let path = self.normalize_file(path)?;
        Ok(Box::new(ZipFileWriter {
            path:   path.into(),
            buffer: Cursor::new(Vec::new()),
            imp:    match self.weak {
                true    => ZipFileWriterRef::Weak(Arc::downgrade(&self.imp)),
                false   => ZipFileWriterRef::Strong(Arc::clone(&self.imp)),
            }
        }))
    }

    fn metadata(&self, path: &str) -> VfsResult<VfsMetadata> {
        let path = self.normalize_path_dir(path)?.0;
        if path == "" {
            Ok(VfsMetadata { file_type: VfsFileType::Directory, len: 0 })
        } else if self.imp.lock().unwrap().dirs.contains(path) {
            Ok(VfsMetadata { file_type: VfsFileType::Directory, len: 0 })
        } else {
            Err(VfsError::FileNotFound { path: path.into() })
        }
    }

    fn exists(&self, path: &str) -> bool {
        let path = match self.normalize_path_dir(path) {
            Err(_) => return false,
            Ok((path, _dir)) => path,
        };
        path == "" || self.imp.lock().unwrap().dirs.contains(path)
    }

    // these all involve reading, which zip::write::ZipWriter doesn't support
    fn read_dir     (&self, _path: &str)            -> VfsResult<Box<dyn Iterator<Item = String>>>  { Err(VfsError::NotSupported) }
    fn open_file    (&self, _path: &str)            -> VfsResult<Box<dyn SeekAndRead>>              { Err(VfsError::NotSupported) }
    fn append_file  (&self, _path: &str)            -> VfsResult<Box<dyn Write>>                    { Err(VfsError::NotSupported) }
    fn remove_file  (&self, _path: &str)            -> VfsResult<()>                                { Err(VfsError::NotSupported) }
    fn remove_dir   (&self, _path: &str)            -> VfsResult<()>                                { Err(VfsError::NotSupported) }
    fn copy_file    (&self, _src: &str, _dst: &str) -> VfsResult<()>                                { Err(VfsError::NotSupported) }
    fn move_file    (&self, _src: &str, _dst: &str) -> VfsResult<()>                                { Err(VfsError::NotSupported) }
    fn move_dir     (&self, _src: &str, _dst: &str) -> VfsResult<()>                                { Err(VfsError::NotSupported) }
}

enum ZipFileWriterRef<IO: Write + Seek + Send + 'static> {
    Weak(Weak<Mutex<Imp<IO>>>),
    Strong(Arc<Mutex<Imp<IO>>>),
}

struct ZipFileWriter<IO: Write + Seek + Send + 'static> {
    path:   String,
    buffer: Cursor<Vec<u8>>,
    imp:    ZipFileWriterRef<IO>,
}

impl<IO: Write + Seek + Send> Write for ZipFileWriter<IO> {
    // Forward all the Write methods I can to the underlying buffer
    fn write                (&mut self, buf: &[u8])             -> io::Result<usize>    { self.buffer.write(buf) }
    fn flush                (&mut self)                         -> io::Result<()>       { self.buffer.flush() }
    fn write_all            (&mut self, buf: &[u8])             -> io::Result<()>       { self.buffer.write_all(buf) }
    fn write_fmt            (&mut self, fmt: fmt::Arguments<'_>)-> io::Result<()>       { self.buffer.write_fmt(fmt) }

    // unstable or missing in 1.34.0
    //fn write_vectored       (&mut self, bufs: &[IoSlice<'_>])   -> io::Result<usize>    { self.buffer.write_vectored(bufs) }
    //fn is_write_vectored    (&self)                             -> bool             { self.buffer.is_write_vectored() }
    //fn write_all_vectored   (&mut self, mut bufs: &mut [IoSlice<'_>]) -> io::Result<()> { self.buffer.write_all_vectored(bufs) }
}

impl<IO: Write + Seek + Send> Drop for ZipFileWriter<IO> {
    fn drop(&mut self) {
        let path    = replace(&mut self.path, String::new());
        let buffer  = replace(&mut self.buffer, Cursor::new(Vec::new())).into_inner();
        let imp     = match replace(&mut self.imp, ZipFileWriterRef::Weak(Weak::default())) {
            ZipFileWriterRef::Strong(s) => s,
            ZipFileWriterRef::Weak(w) => match w.upgrade() {
                Some(s) => s,
                None => return,
            }
        };
        let mut imp = imp.lock().unwrap();
        if imp.writer.start_file(path, zip::write::FileOptions::default()).is_err() { return; }
        let _ = imp.writer.write_all(&buffer[..]);
    }
}

#[cfg(test)] mod tests {
    use crate::*;
    use super::VfsPath;
    use std::fs::{create_dir_all, File};

    #[test] fn copy_early_vfs_zip() {
        let _ = create_dir_all("target/tmp");
        let src = VfsPath::new(ZipReadOnly::new_strict(File::open("test/data/early-vfs-zip.zip").unwrap()).unwrap());
        let dst = VfsPath::new(ZipWriteOnly::new_weak(File::create("target/tmp/early-vfs-zip-copy.zip").unwrap()).unwrap());
        // NOTE: https://github.com/MaulingMonkey/vfs-zip/issues/1
        let copied = src.copy_dir(&dst.join("subdir").unwrap()).unwrap();
        assert_eq!(copied, 16);
    }
}
