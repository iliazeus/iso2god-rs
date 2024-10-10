use std::io::{Read, Seek, SeekFrom};
use std::ops::{Bound, Range, RangeBounds};

pub trait ReadFromSlice<R: Read + Seek>
where
    Self: Sized,
{
    type Error: std::error::Error;
    fn read_from_slice(rs: &mut ReadSlice<R>) -> Result<Self, Self::Error>;
}

#[derive(Debug, Clone)]
pub struct ReadSlice<R> {
    reader: R,
    range: Range<u64>,
}

impl<R> ReadSlice<R> {
    pub fn range(&self) -> Range<u64> {
        self.range.clone()
    }

    pub fn from_range(reader: R, range: Range<u64>) -> Self {
        Self { reader, range }
    }

    pub fn into_inner(self) -> R {
        self.reader
    }

    pub fn by_ref(&mut self) -> ReadSlice<&mut R> {
        ReadSlice {
            reader: &mut self.reader,
            range: self.range.clone(),
        }
    }

    /// Returns a copy, unlike [Self::narrow].
    pub fn slice(self, subrange: impl RangeBounds<u64>) -> Self {
        let new_start = match subrange.start_bound() {
            Bound::Included(x) => self.range.start + *x,
            Bound::Excluded(x) => self.range.start + *x + 1,
            Bound::Unbounded => self.range.start,
        };

        let new_end = match subrange.end_bound() {
            Bound::Included(x) => self.range.start + *x + 1,
            Bound::Excluded(x) => self.range.start + *x,
            Bound::Unbounded => self.range.end,
        };

        Self {
            reader: self.reader,
            range: new_start..new_end,
        }
    }

    /// Modifies `self`, unlike [Self::slice].
    /// This method is more convenient in some cases, but a potential footgun.
    pub fn narrow(&mut self, subrange: impl RangeBounds<u64>) -> &mut Self {
        let new_start = match subrange.start_bound() {
            Bound::Included(x) => self.range.start + *x,
            Bound::Excluded(x) => self.range.start + *x + 1,
            Bound::Unbounded => self.range.start,
        };

        let new_end = match subrange.end_bound() {
            Bound::Included(x) => self.range.start + *x + 1,
            Bound::Excluded(x) => self.range.start + *x,
            Bound::Unbounded => self.range.end,
        };

        self.range = new_start..new_end;
        self
    }
}

impl<R: Read + Seek> ReadSlice<R> {
    pub fn read<T: ReadFromSlice<R>>(&mut self) -> Result<T, T::Error> {
        T::read_from_slice(self)
    }

    pub fn from_whole(mut reader: R) -> Result<Self, std::io::Error> {
        let end = reader.seek(SeekFrom::End(0))?;
        Ok(Self {
            reader,
            range: 0..end,
        })
    }

    pub fn seek_to_start(mut self) -> Result<R, std::io::Error> {
        self.reader.seek(SeekFrom::Start(self.range.start))?;
        Ok(self.reader)
    }

    pub fn seek_to_end(mut self) -> Result<R, std::io::Error> {
        self.reader.seek(SeekFrom::Start(self.range.end))?;
        Ok(self.reader)
    }

    pub fn seek_to_offset(mut self, offset: u64) -> Result<R, std::io::Error> {
        self.reader
            .seek(SeekFrom::Start(self.range.start + offset))?;
        Ok(self.reader)
    }

    pub fn take_whole(mut self) -> Result<std::io::Take<R>, std::io::Error> {
        self.reader.seek(SeekFrom::Start(self.range.start))?;
        Ok(self.reader.take(self.range.end - self.range.start))
    }

    pub fn assert_magic<const N: usize>(mut self, magic: &[u8; N]) -> Result<Self, std::io::Error> {
        self.reader.seek(SeekFrom::Start(self.range.start))?;

        let mut buf = [0u8; N];
        self.reader.read_exact(&mut buf)?;

        if &buf == magic {
            Ok(self)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "magic bytes not found",
            ))
        }
    }
}
