use crate::{Error, Result};
use super::{ReadAtCursor, ReadAtLen};

use read_write_at::ReadAt;

use std::collections::*;
use std::convert::*;
use std::fmt::{self, Debug, Formatter};
use std::path::*;



/// A read-only zip archive filesystem
pub struct ZipReadOnly<IO: ReadAt> {
    pub(super) io:     IO,
    pub(super) files:  BTreeMap<String, FileEntry>, // abs path -> ...
    pub(super) dirs:   BTreeMap<String, BTreeSet<String>>, // abs path -> [relative file/dir names]
}

pub(super) struct FileEntry {
    pub header_offset:  u64,
    pub header_size:    u64,
    pub compressed:     u64,
    pub uncompressed:   u64,
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
