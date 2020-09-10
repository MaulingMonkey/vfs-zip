use super::{SeeklessFile, SeeklessSharedIO, SeeklessBlob};

use read_write_at::ReadAt;

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex};



/// Convert into a (cheaply!) [Clone]able [ReadAt] + [u64] length.  Implementations include:<br>
/// [File], [Vec]\<u8\>, [Arc]\<\[u8\]\>, [Box]\<\[u8\]\>, \&\[u8\], [Mutex]\<[Read] + [Seek]\>, and [Arc]\<[Mutex]\<[Read] + [Seek]\>\>.
///
/// [ReadAt]:   https://docs.rs/read_write_at/0.1.0/read_write_at/trait.ReadAt.html
pub trait IntoCloneReadAtLen {
    /// Some [Clone]able [ReadAt] implementation.
    ///
    /// [ReadAt]:   https://docs.rs/read_write_at/0.1.0/read_write_at/trait.ReadAt.html
    type ReadAt : Clone + ReadAt;

    /// Acquire a [Clone]able [ReadAt] + [u64] length
    ///
    /// [ReadAt]:   https://docs.rs/read_write_at/0.1.0/read_write_at/trait.ReadAt.html
    fn into_read_at_len(self) -> io::Result<(Self::ReadAt, u64)>;
}

#[cfg(any(target_os = "redox", unix, target_os = "vxworks", target_os = "hermit", windows))]
impl IntoCloneReadAtLen for File {
    type ReadAt = SeeklessFile;
    fn into_read_at_len(mut self) -> io::Result<(Self::ReadAt, u64)> {
        let len = self.seek(SeekFrom::End(0))?;
        Ok((SeeklessFile::from(self), len))
    }
}

#[cfg(not(any(target_os = "redox", unix, target_os = "vxworks", target_os = "hermit", windows)))]
impl IntoCloneReadAtLen for File {
    type ReadAt = SeeklessSharedIO<File>;
    fn into_read_at_len(mut self) -> io::Result<(Self::ReadAt, u64)> {
        let len = self.seek(SeekFrom::End(0))?;
        Ok((SeeklessSharedIO::from(self), len))
    }
}

impl IntoCloneReadAtLen for Vec<u8> {
    type ReadAt = SeeklessBlob<Arc<[u8]>>;
    fn into_read_at_len(self) -> io::Result<(Self::ReadAt, u64)> {
        let arc = Arc::<[u8]>::from(self);
        let len = arc.len() as u64;
        Ok((SeeklessBlob(arc), len))
    }
}

impl IntoCloneReadAtLen for Arc<[u8]> {
    type ReadAt = SeeklessBlob<Arc<[u8]>>;
    fn into_read_at_len(self) -> io::Result<(Self::ReadAt, u64)> {
        let len = self.as_ref().len() as u64;
        Ok((SeeklessBlob(self), len))
    }
}

impl IntoCloneReadAtLen for Box<[u8]> {
    type ReadAt = SeeklessBlob<Box<[u8]>>;
    fn into_read_at_len(self) -> io::Result<(Self::ReadAt, u64)> {
        let len = self.as_ref().len() as u64;
        Ok((SeeklessBlob(self), len))
    }
}

impl<'s> IntoCloneReadAtLen for &'s [u8] {
    type ReadAt = SeeklessBlob<&'s [u8]>;
    fn into_read_at_len(self) -> io::Result<(Self::ReadAt, u64)> {
        let len = self.len() as u64;
        Ok((SeeklessBlob(self), len))
    }
}

impl<'s> IntoCloneReadAtLen for &'s mut [u8] {
    type ReadAt = SeeklessBlob<&'s [u8]>;
    fn into_read_at_len(self) -> io::Result<(Self::ReadAt, u64)> {
        let len = self.len() as u64;
        Ok((SeeklessBlob(self), len))
    }
}

impl<IO: Read + Seek> IntoCloneReadAtLen for Mutex<IO> {
    type ReadAt = SeeklessSharedIO<IO>;
    fn into_read_at_len(self) -> io::Result<(Self::ReadAt, u64)> {
        let len = self.lock().map_err(|_| io::Error::new(io::ErrorKind::Other, "mutex was poisoned"))?.seek(SeekFrom::End(0))?;
        Ok((SeeklessSharedIO::from(self), len))
    }
}

impl<IO: Read + Seek> IntoCloneReadAtLen for Arc<Mutex<IO>> {
    type ReadAt = SeeklessSharedIO<IO>;
    fn into_read_at_len(self) -> io::Result<(Self::ReadAt, u64)> {
        let len = self.lock().map_err(|_| io::Error::new(io::ErrorKind::Other, "mutex was poisoned"))?.seek(SeekFrom::End(0))?;
        Ok((SeeklessSharedIO::from(self), len))
    }
}
