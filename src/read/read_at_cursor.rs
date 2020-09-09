use read_write_at::ReadAt;

use std::io::{self, Read, Seek, SeekFrom};



/// Adapt read_write_at::ReadAt back into std::io::Read and std::io::Seek
pub(crate) struct ReadAtCursor<'ra, RA: ReadAt> {
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
