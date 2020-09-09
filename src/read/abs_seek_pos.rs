//use std::convert::From;
use std::io::{self, Error, ErrorKind, SeekFrom};

pub(crate) struct AbsSeekPos(pub u64);

impl AbsSeekPos {
    /// Result: Ok( 0 ..= end ) or Error (kind() == InvalidInput)
    pub fn offset_bounded(&self, pos: SeekFrom, end: u64) -> io::Result<AbsSeekPos> {
        let cur = self.0;
        Ok(Self(match pos {
            SeekFrom::Start(n)      => n,
            SeekFrom::End(n)        => Self::add(end, n)?,
            SeekFrom::Current(n)    => Self::add(cur, n)?,
        }))
    }

    #[allow(dead_code)] // XXX
    /// Result: Ok( 0 ..= std::u64::MAX ) or Error (kind() == InvalidInput)
    pub fn offset_unbounded(&self, pos: SeekFrom, end: u64) -> io::Result<AbsSeekPos> {
        let cur = self.0;
        Ok(Self(match pos {
            SeekFrom::Start(n)      => n,
            SeekFrom::End(n)        => Self::add(end, n)?,
            SeekFrom::Current(n)    => Self::add(cur, n)?,
        }))
    }

    fn add(cur: u64, off: i64) -> io::Result<u64> {
        Ok(match PZN::from(off) {
            PZN::Positive(n)    => cur.checked_add(n).ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Attempted to seek before start of stream"))?,
            PZN::Zero           => cur,
            PZN::Negative(n)    => cur.checked_sub(n).ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Attempted to seek past end of stream"))?,
        })
    }
}

enum PZN {
    Positive(u64),
    Zero,
    Negative(u64)
}

impl From<i64> for PZN {
    fn from(value: i64) -> Self {
        if value > 0 {
            PZN::Positive(value as u64)
        } else if value < 0 {
            PZN::Negative(0u64.wrapping_sub(value as u64)) // from(-42) == Negative(0 - - 42) == Negative(42)
        } else {
            PZN::Zero
        }
    }
}
