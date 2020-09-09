use crate::AbsSeekPos;

use read_write_at::ReadAt;

use std::io::{self, Seek, SeekFrom, Read};
use std::ops::Range;



/// Adapt [ReadAt] / [Read] / [Seek] into a subrange of the original IO
pub(crate) struct ReadRange<IO> {
    io:         IO,
    start:      u64,
    length:     u64,
    seek:       AbsSeekPos,
}

impl<IO> ReadRange<IO> {
    pub fn new(io: IO, range: Range<u64>) -> Self {
        Self {
            io,
            start:      range.start,
            length:     range.end - range.start,
            seek:       AbsSeekPos(std::u64::MAX),
        }
    }
}

impl<IO: Seek> Seek for ReadRange<IO> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        if self.seek.0 == std::u64::MAX { self.seek.0 = 0; }
        self.seek = self.seek.offset_bounded(pos, self.length)?;
        self.io.seek(SeekFrom::Start(self.start + self.seek.0)).map(|o| o - self.start)
    }
}

impl<IO: Read + Seek> Read for ReadRange<IO> {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        if self.seek.0 == std::u64::MAX { self.seek(SeekFrom::Start(0))?; }

        let remaining = self.length - self.seek.0;
        if buf.len() as u64 > remaining {
            buf = &mut buf[..remaining as usize];
        }
        let read = self.io.read(buf)?;
        self.seek.0 += read as u64;
        Ok(read)
    }
}

impl<IO: ReadAt> ReadAt for ReadRange<IO> {
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        if offset > self.length {
            Err(io::Error::new(io::ErrorKind::InvalidInput, "Attempted to read past end of stream"))
        } else {
            let len = (buf.len() as u64).min(self.length - offset) as usize;
            self.io.read_at(&mut buf[..len], offset + self.start)
        }
    }
}
