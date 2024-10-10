use std::io::{self, Read, Seek, SeekFrom};
use std::marker::PhantomData;
use std::ops::{Bound, RangeBounds};

pub trait ReadFromRange
where
    Self: Sized,
{
    fn read_from_range<R: Read + Seek>(r: R, off: u64, len: u64) -> io::Result<Self>;

    fn read_whole<R: Read + Seek>(mut r: R) -> io::Result<(RangeRef<Self>, Self)> {
        let length = r.seek(SeekFrom::End(0))?;
        let value = Self::read_from_range(r, 0, length)?;
        let range_ref = RangeRef {
            _value: PhantomData,
            offset: 0,
            length,
        };
        Ok((range_ref, value))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RangeRef<T> {
    _value: PhantomData<T>,
    offset: u64,
    length: u64, // TODO: should we use `end` instead?
}

impl<T> Copy for RangeRef<T> {}
impl<T> Clone for RangeRef<T> {
    fn clone(&self) -> Self {
        Self {
            _value: PhantomData,
            offset: self.offset,
            length: self.length,
        }
    }
}

impl<T> RangeRef<T> {
    pub fn whole<R: Read + Seek>(mut r: R) -> io::Result<Self> {
        let length = r.seek(SeekFrom::End(0))?;
        Ok(Self {
            _value: PhantomData,
            offset: 0,
            length,
        })
    }
}

impl<T: ReadFromRange> RangeRef<T> {
    pub fn read<R: Read + Seek>(&self, r: R) -> io::Result<T> {
        T::read_from_range(r, self.offset, self.length)
    }
}

impl<T> RangeRef<T> {
    pub fn slice<T2>(&self, range: impl RangeBounds<u64>) -> RangeRef<T2> {
        let offset = match range.start_bound() {
            Bound::Included(x) => self.offset + x,
            Bound::Excluded(x) => self.offset + x + 1,
            Bound::Unbounded => self.offset,
        };

        let end = match range.end_bound() {
            Bound::Included(x) => self.offset + x + 1,
            Bound::Excluded(x) => self.offset + x,
            Bound::Unbounded => self.offset + self.length,
        };

        let length = end - offset;
        debug_assert!(self.length >= length);

        RangeRef {
            _value: PhantomData,
            offset,
            length,
        }
    }
}
