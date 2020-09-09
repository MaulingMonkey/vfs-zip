use read_write_at::ReadAt;

use std::io::{self, Seek, SeekFrom, Read};
use std::sync::{Arc, Mutex};



/// Implementation detail of impl IntoCloneReadAtLen for std::fs::File
#[doc(hidden)]
#[derive(Clone)]
pub struct SeeklessFile(Arc<std::fs::File>);

impl From<std::fs::File>      for SeeklessFile { fn from(file:     std::fs::File ) -> Self { Self(Arc::new(file)) } }
impl From<Arc<std::fs::File>> for SeeklessFile { fn from(file: Arc<std::fs::File>) -> Self { Self(file) } }

#[cfg(any(target_os = "redox", unix, target_os = "vxworks", target_os = "hermit"))] impl ReadAt for SeeklessFile {
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> { self.0.read_at(buf, offset) }
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> { self.0.read_exact_at(buf, offset) }
}

#[cfg(windows)] impl ReadAt for SeeklessFile {
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        std::os::windows::fs::FileExt::seek_read(&*self.0, buf, offset)
    }
}



/// Implementation detail of impl IntoCloneReadAtLen for Mutex<IO>
#[doc(hidden)]
pub struct SeeklessSharedIO<IO>(Arc<Mutex<IO>>);

impl<IO> Clone for SeeklessSharedIO<IO> { fn clone(&self) -> Self { Self(Arc::clone(&self.0)) } }

impl<IO> From<Arc<Mutex<IO>>> for SeeklessSharedIO<IO> { fn from(io: Arc<Mutex<IO>>) -> Self { Self(io) } }
impl<IO> From<    Mutex<IO> > for SeeklessSharedIO<IO> { fn from(io:     Mutex<IO> ) -> Self { Self(Arc::new(io)) } }
impl<IO> From<          IO  > for SeeklessSharedIO<IO> { fn from(io:           IO  ) -> Self { Self(Arc::new(Mutex::new(io))) } }

impl<IO: Seek + Read> ReadAt for SeeklessSharedIO<IO> {
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        let mut io = self.0.lock().unwrap();
        io.seek(SeekFrom::Start(offset))?;
        io.read(buf)
    }

    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        let mut io = self.0.lock().unwrap();
        io.seek(SeekFrom::Start(offset))?;
        io.read_exact(buf)
    }
}



/// Implementation detail of impl IntoCloneReadAtLen for Vec<u8> / Arc<[u8]> / &[u8]
#[doc(hidden)]
#[derive(Clone)]
pub struct SeeklessBlob<B: AsRef<[u8]>>(pub(crate) B);

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
