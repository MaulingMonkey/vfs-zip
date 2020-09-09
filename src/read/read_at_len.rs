use super::{SeeklessFile, SeeklessBlob};

use read_write_at::{ReadAt, ReadWriteSeek};

use std::io::{self, Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex};



/// Convert a Mutex\<[Read] + [Seek]\> / Vec\<u8\> / \&\[u8\] / [File] (on most platforms) into a [ReadAt] + [u64] length.
///
/// [ReadAt]:           https://docs.rs/read_write_at/0.1.0/read_write_at/trait.ReadAt.html
/// [std::fs::File]:    https://doc.rust-lang.org/std/fs/struct.File.html
/// [File]:             https://doc.rust-lang.org/std/fs/struct.File.html
pub trait ReadAtLen {
    /// Some [ReadAt](https://docs.rs/read_write_at/0.1.0/read_write_at/trait.ReadAt.html) implementation.
    type ReadAt : ReadAt;

    /// Acquire a [ReadAt]
    ///
    /// [ReadAt]:   https://docs.rs/read_write_at/0.1.0/read_write_at/trait.ReadAt.html
    fn into_read_at_len(self) -> io::Result<(Self::ReadAt, u64)>;
}

#[cfg(any(target_os = "redox", unix, target_os = "vxworks", target_os = "hermit", windows))]
impl ReadAtLen for std::fs::File {
    type ReadAt = SeeklessFile;
    fn into_read_at_len(mut self) -> io::Result<(Self::ReadAt, u64)> {
        let len = self.seek(SeekFrom::End(0))?;
        Ok((SeeklessFile(self), len))
    }
}

#[cfg(not(any(target_os = "redox", unix, target_os = "vxworks", target_os = "hermit", windows)))]
impl ReadAtLen for std::fs::File {
    type ReadAt = Mutex<Self>;
    fn into_read_at_len(self) -> io::Result<(Self::ReadAt, u64)> {
        let len = self.seek(SeekFrom::End(0))?;
        Ok((Mutex::new(self), len))
    }
}

impl<IO: Read + Seek> ReadAtLen for Mutex<IO> {
    type ReadAt = Mutex<ReadWriteSeek<IO>>;
    fn into_read_at_len(self) -> io::Result<(Self::ReadAt, u64)> {
        let mut inner = self.into_inner().map_err(|_| io::Error::new(io::ErrorKind::Other, "mutex was poisoned"))?;
        let len = inner.seek(SeekFrom::End(0))?;
        Ok((Mutex::new(ReadWriteSeek(inner)), len))
    }
}

impl ReadAtLen for Vec<u8> {
    type ReadAt = SeeklessBlob<Vec<u8>>;
    fn into_read_at_len(self) -> io::Result<(Self::ReadAt, u64)> {
        let len = Vec::len(&self) as u64;
        Ok((SeeklessBlob(self), len))
    }
}

impl ReadAtLen for Arc<[u8]> {
    type ReadAt = SeeklessBlob<Arc<[u8]>>;
    fn into_read_at_len(self) -> io::Result<(Self::ReadAt, u64)> {
        let len = self.as_ref().len() as u64;
        Ok((SeeklessBlob(self), len))
    }
}

impl ReadAtLen for Box<[u8]> {
    type ReadAt = SeeklessBlob<Box<[u8]>>;
    fn into_read_at_len(self) -> io::Result<(Self::ReadAt, u64)> {
        let len = self.as_ref().len() as u64;
        Ok((SeeklessBlob(self), len))
    }
}

impl<'s> ReadAtLen for &'s [u8] {
    type ReadAt = SeeklessBlob<&'s [u8]>;
    fn into_read_at_len(self) -> io::Result<(Self::ReadAt, u64)> {
        let len = self.len() as u64;
        Ok((SeeklessBlob(self), len))
    }
}

impl<'s> ReadAtLen for &'s mut [u8] {
    type ReadAt = SeeklessBlob<&'s [u8]>;
    fn into_read_at_len(self) -> io::Result<(Self::ReadAt, u64)> {
        let len = self.len() as u64;
        Ok((SeeklessBlob(self), len))
    }
}
