use std::fmt::{self, Debug, Display, Formatter};
use zip::result::ZipError;



/// A generic opaque vfs-zip error
pub struct Error(pub(crate) ZipError);
// Newtype: I might want to switch away from `zip` in the future to implement
// multiple file access?  Or bump `zip` versions without breaking semver changes?
// Either way, this avoids exposing `zip` types directly.

impl Debug   for Error { fn fmt(&self, fmt: &mut Formatter) -> fmt::Result { Debug  ::fmt(&self.0, fmt) } }
impl Display for Error { fn fmt(&self, fmt: &mut Formatter) -> fmt::Result { Display::fmt(&self.0, fmt) } }
impl std::error::Error for Error {}
impl Error {
    pub(crate) fn unsupported(s: &'static str) -> Self { Self(ZipError::UnsupportedArchive(s)) }
    pub(crate) fn io(io: std::io::Error) -> Self { Self(ZipError::Io(io)) }
}

#[cfg(feature = "vfs04")]
pub(crate) fn zip2vfs(path: &str, e: ZipError) -> vfs04::VfsError {
    use vfs04::VfsError;
    use std::io::{Error as IoError, ErrorKind::InvalidData};

    match e {
        ZipError::FileNotFound          => VfsError::FileNotFound { path: path.into() },
        ZipError::Io(e)                 => VfsError::IoError(e),
        ZipError::InvalidArchive(e)     => VfsError::IoError(IoError::new(InvalidData, e)),
        ZipError::UnsupportedArchive(e) => VfsError::IoError(IoError::new(InvalidData, e)),
        other                           => VfsError::Other { message: other.to_string() },
    }
}



/// Shorthand for [std::result::Result]<T, vfs_zip::[Error]>
pub type Result<T> = std::result::Result<T, Error>;
