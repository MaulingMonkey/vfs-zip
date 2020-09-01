#[cfg(feature = "vfs04")] mod vfs04;

use crate::{Error, Result};

use std::collections::*;
use std::fmt::{self, Debug, Formatter};
use std::io::{Read, Seek};
use std::path::*;
use std::sync::Mutex;



/// A read-only zip archive filesystem
pub struct ZipReadOnly<IO: Read + Seek + Send + 'static> {
    archive:    Mutex<zip::read::ZipArchive<IO>>,
    files:      BTreeMap<String, usize>,            // abs path -> zip archive index
    dirs:       BTreeMap<String, BTreeSet<String>>, // abs path -> [relative file/dir names]
}

impl<IO: Read + Seek + Send> Debug for ZipReadOnly<IO> {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(fmt, "ZipReadOnly")
    }
}

impl<IO: Read + Seek + Send> ZipReadOnly<IO> {
    /// Create a new read-only zip filesystem.
    /// Any archive errors (including unsupported paths) will result in errors.
    pub fn new_strict(io: IO) -> Result<Self> { Self::new_impl(io, false) }

    /// Create a new read-only zip filesystem.
    /// Some archive errors (such as unsupported paths) will be ignored.
    pub fn new_relaxed(io: IO) -> Result<Self> { Self::new_impl(io, true) }

    fn new_impl(io: IO, ignore_file_errors: bool) -> Result<Self> {
        let mut za = Self {
            archive:    Mutex::new(zip::read::ZipArchive::new(io).map_err(Error)?),
            files:      Default::default(),
            dirs:       Default::default(),
        };
        za.dirs.insert(String::new(), Default::default()); // always have a root directory

        let mut archive = za.archive.lock().unwrap();
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
                if za.files.insert(abs.into(), i).is_some() { continue 'files; } // already inserted
            } else if entry.is_dir() {
                za.dirs.entry(abs.into()).or_default();
            }

            while let Some(slash) = abs.rfind('/') {
                let dir_name = &abs[..slash];
                let leaf_name = &abs[slash+1..];

                let dir = za.dirs.entry(dir_name.into()).or_default();
                if !dir.insert(leaf_name.into()) { continue 'files; } // already inserted

                abs = dir_name;
            }

            let root = za.dirs.get_mut("").unwrap();
            root.insert(abs.into());
        }

        std::mem::drop(archive); // unlock
        Ok(za)
    }
}
