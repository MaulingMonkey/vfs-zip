use super::AbsSeekPos;

use read_write_at::ReadAt;

use std::io::{self, Read, Seek, SeekFrom};



/// Adapt [ReadAt] back into [Read] + [Seek]
#[derive(Clone)]
pub(crate) struct ReadAtCursor<RA: ReadAt> {
    offset: u64,
    length: u64,
    ra:     RA,
}

impl<RA: ReadAt> ReadAtCursor<RA> {
    pub fn new(ra: RA, length: u64) -> Self {
        Self {
            offset: 0,
            length,
            ra,
        }
    }
}

impl<RA: ReadAt> Read for ReadAtCursor<RA> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read = self.ra.read_at(buf, self.offset)?;
        self.offset = self.offset.checked_add(read as u64).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Attempted to read past 18.45 EB"))?;
        Ok(read)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        let dst = self.offset.checked_add(buf.len() as u64).ok_or_else(|| io::ErrorKind::UnexpectedEof)?;
        self.ra.read_exact_at(buf, self.offset)?;
        self.offset = dst;
        Ok(())
    }
}

impl<RA: ReadAt> Seek for ReadAtCursor<RA> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.offset = AbsSeekPos(self.offset).offset_bounded(pos, self.length)?.0;
        Ok(self.offset)
    }
}
