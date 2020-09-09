#[cfg(feature = "vfs04")] mod vfs04;

use crate::{Error, Result};

use read_write_at::{ReadAt, ReadWriteSeek};

use std::collections::*;
use std::convert::*;
use std::fmt::{self, Debug, Formatter};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::*;
use std::sync::Mutex;



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

#[cfg(any(unix, windows))]
impl ReadAtLen for std::fs::File {
    type ReadAt = SeeklessFile;
    fn into_read_at_len(mut self) -> io::Result<(Self::ReadAt, u64)> {
        let len = self.seek(SeekFrom::End(0))?;
        Ok((SeeklessFile(self), len))
    }
}

#[cfg(not(any(unix, windows)))]
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

impl<'s> ReadAtLen for &'s [u8] {
    type ReadAt = SeeklessBlob<&'s [u8]>;
    fn into_read_at_len(self) -> io::Result<(Self::ReadAt, u64)> {
        let len = self.len() as u64;
        Ok((SeeklessBlob(self), len))
    }
}



/// A read-only zip archive filesystem
pub struct ZipReadOnly<IO: ReadAt> {
    io:     IO,
    files:  BTreeMap<String, FileEntry>, // abs path -> ...
    dirs:   BTreeMap<String, BTreeSet<String>>, // abs path -> [relative file/dir names]
}

struct FileEntry {
    header_offset:  u64,
    header_size:    u64,
    compressed:     u64,
    uncompressed:   u64,
}

impl<IO: ReadAt> Debug for ZipReadOnly<IO> {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(fmt, "ZipReadOnly")
    }
}

impl<IO: ReadAt> ZipReadOnly<IO> {
    /// Create a new read-only zip filesystem.
    /// Any archive errors (including unsupported paths) will result in errors.
    pub fn new_strict(ral: impl ReadAtLen<ReadAt = IO>) -> Result<Self> { Self::new_impl(ral, false) }

    /// Create a new read-only zip filesystem.
    /// Some archive errors (such as unsupported paths) will be ignored.
    pub fn new_relaxed(ral: impl ReadAtLen<ReadAt = IO>) -> Result<Self> { Self::new_impl(ral, true) }

    fn new_impl(ral: impl ReadAtLen<ReadAt = IO>, ignore_file_errors: bool) -> Result<Self> {
        let (ra, len) = ral.into_read_at_len().map_err(Error::io)?;
        let mut zro = Self {
            io: ra,
            files:  Default::default(),
            dirs:   Default::default(),
        };

        let mut archive = zip::read::ZipArchive::new(ReadAtCursor::new(&zro.io, len)).map_err(Error)?;
        zro.dirs.insert(String::new(), Default::default()); // always have a root directory

        'files: for i in 0..archive.len() {
            let entry = archive.by_index(i);
            if ignore_file_errors && entry.is_err() { continue; }
            let entry = entry.map_err(Error)?;
            let abs = entry.name();
            if abs.contains('\\')           { if ignore_file_errors { continue } return Err(Error::unsupported("vfs-zip doesn't support zip archives containing backslashes in paths")); }
            if abs.contains("//")           { if ignore_file_errors { continue } return Err(Error::unsupported("vfs-zip doesn't support zip archives containing 0-length directory names")); }
            let mut abs = abs.trim_end_matches('/');
            if Path::new(abs).is_absolute() { if ignore_file_errors { continue } return Err(Error::unsupported("vfs-zip doesn't support zip archives containing absolute paths")); }

            if entry.is_file() {
                let entry = FileEntry {
                    header_offset:  entry.header_start(),
                    header_size:    entry.data_start() - entry.header_start(),
                    compressed:     entry.compressed_size(),
                    uncompressed:   entry.size(),
                };
                if zro.files.insert(abs.into(), entry).is_some() { continue 'files; } // already inserted
            } else if entry.is_dir() {
                zro.dirs.entry(abs.into()).or_default();
            }

            while let Some(slash) = abs.rfind('/') {
                let dir_name = &abs[..slash];
                let leaf_name = &abs[slash+1..];

                let dir = zro.dirs.entry(dir_name.into()).or_default();
                if !dir.insert(leaf_name.into()) { continue 'files; } // already inserted

                abs = dir_name;
            }

            let root = zro.dirs.get_mut("").unwrap();
            root.insert(abs.into());
        }

        std::mem::drop(archive); // unlock
        Ok(zro)
    }
}



/// Implementation detail of impl ReadAtLen for std::fs::File
#[doc(hidden)]
pub struct SeeklessFile(std::fs::File);

#[cfg(unix)] impl ReadAt for SeeklessFile {
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> { self.0.read_at(buf, offset) }
    fn read_exact_at(&self, mut buf: &mut [u8], mut offset: u64) -> io::Result<()> { self.0.read_exact_at(buf, offset) }
}

#[cfg(windows)] impl ReadAt for SeeklessFile {
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        std::os::windows::fs::FileExt::seek_read(&self.0, buf, offset)
    }
}



/// Implementation detail of impl ReadAtLen for Vec<u8> / &[u8]
#[doc(hidden)]
pub struct SeeklessBlob<B: AsRef<[u8]>>(B);

impl<B: AsRef<[u8]>> ReadAt for SeeklessBlob<B> {
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        let src = self.0.as_ref();
        if offset >= src.len() as u64 {
            Err(io::Error::new(io::ErrorKind::InvalidInput, "Tried to read past end of SeeklessBlob"))
        } else {
            let offset = offset as usize; // was smaller than len which was usize
            let n = buf.len().min(src.len() - offset);
            buf[..n].copy_from_slice(&src[offset..(offset+n)]);
            Ok(n)
        }
    }
}



/// Adapt read_write_at::ReadAt back into std::io::Read and std::io::Seek
struct ReadAtCursor<'ra, RA: ReadAt> {
    offset: u64,
    length: u64,
    ra:     &'ra RA,
}

impl<'ra, RA: ReadAt> ReadAtCursor<'ra, RA> {
    pub fn new(ra: &'ra RA, length: u64) -> Self {
        Self {
            offset: 0,
            length,
            ra,
        }
    }
}

impl<RA: ReadAt> Read for ReadAtCursor<'_, RA> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read = self.ra.read_at(buf, self.offset)?;
        self.offset += read as u64;
        Ok(read)
    }

    fn read_exact(&mut self, mut buf: &mut [u8]) -> io::Result<()> {
        while !buf.is_empty() {
            let read = self.ra.read_at(buf, self.offset)?;
            if read == 0 { break }

            buf = &mut buf[read..];
            self.offset += read as u64;
        }
        Ok(())
    }
}

impl<RA: ReadAt> Seek for ReadAtCursor<'_, RA> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match pos {
            SeekFrom::Start(n) => {
                if n > self.length {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, "Tried to seek() past ReadAtCursor length"));
                } else {
                    self.offset = n;
                }
            },
            SeekFrom::End(n) => {
                if n > 0 {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, "Tried to seek() past end of ReadAtCursor"));
                } else { // n <= 0
                    let neg_n = 0u64.wrapping_sub(n as u64);
                    let target = self.length.checked_sub(neg_n).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Tried to seek() before start of ReadAtCursor"))?;
                    self.offset = target;
                }
            },
            SeekFrom::Current(n) => {
                if n >= 0 {
                    let target = self.offset.checked_add(n as u64).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Tried to seek() past u64 capacity"))?;
                    if target > self.length { return Err(io::Error::new(io::ErrorKind::InvalidInput, "Tried to seek() past ReadAtCursor length")); }
                    self.offset = target;
                } else { // n < 0
                    let neg_n = 0u64.wrapping_sub(n as u64);
                    let target = self.offset.checked_sub(neg_n).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Tried to seek() before start of ReadAtCursor"))?;
                    self.offset = target;
                }
            },
        }
        Ok(self.offset)
    }
}
