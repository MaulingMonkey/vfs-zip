use read_write_at::ReadAt;
use std::io;

/// &ReadAt, but also implements ReadAt
/// 
/// This should probably be replaced with an upstream `impl<RA: ReadAt> ReadAt for &RA { ... }`
pub(crate) struct ReadAtRef<'ra, RA: ReadAt>(pub &'ra RA);

impl<'ra, RA: ReadAt> ReadAt for ReadAtRef<'ra, RA> {
    fn read_at      (&self, buf: &mut [u8], offset: u64) -> io::Result<usize> { self.0.read_at(buf, offset) }
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()>    { self.0.read_exact_at(buf, offset) }
}
