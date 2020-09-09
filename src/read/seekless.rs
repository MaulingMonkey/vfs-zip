use read_write_at::ReadAt;

use std::io;



/// Implementation detail of impl ReadAtLen for std::fs::File
#[doc(hidden)]
pub struct SeeklessFile(pub(crate) std::fs::File);

#[cfg(any(target_os = "redox", unix, target_os = "vxworks", target_os = "hermit"))] impl ReadAt for SeeklessFile {
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> { self.0.read_at(buf, offset) }
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> { self.0.read_exact_at(buf, offset) }
}

#[cfg(windows)] impl ReadAt for SeeklessFile {
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        std::os::windows::fs::FileExt::seek_read(&self.0, buf, offset)
    }
}



/// Implementation detail of impl ReadAtLen for Vec<u8> / &[u8]
#[doc(hidden)]
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
