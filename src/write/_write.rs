#[cfg(feature = "vfs04")] mod vfs04;

use crate::Result;

use std::collections::*;
use std::fmt::{self, Debug, Formatter};
use std::io::{Write, Seek};
use std::sync::{Arc, Mutex};



/// A write-only zip archive filesystem
pub struct ZipWriteOnly<IO: Write + Seek + Send + 'static> {
    imp:    Arc<Mutex<Imp<IO>>>,
    weak:   bool,
}

struct Imp<IO: Write + Seek + Send + 'static> {
    writer: zip::write::ZipWriter<IO>,
    dirs:   BTreeSet<String>,
}

impl<IO: Write + Seek + Send> Debug for ZipWriteOnly<IO> {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(fmt, "ZipWriteOnly")
    }
}

impl<IO: Write + Seek + Send> ZipWriteOnly<IO> {
    /// Create a new write-only zip filesystem.
    ///
    /// The underlying I/O will not be closed until the filesystem and all outstanding files are dropped.
    pub fn new_strong(io: IO) -> Result<Self> { Self::new_impl(io, false) }

    /// Create a new write-only zip filesystem.
    ///
    /// The underlying I/O will be closed when the filesystem is dropped.
    /// Any outstanding files will start generating I/O errors and will not be committed to the .zip
    pub fn new_weak(io: IO) -> Result<Self> { Self::new_impl(io, true) }

    fn new_impl(io: IO, weak: bool) -> Result<Self> {
        Ok(Self {
            imp: Arc::new(Mutex::new(Imp {
                writer: zip::write::ZipWriter::new(io),
                dirs:   Default::default(),
            })),
            weak,
        })
    }
}
