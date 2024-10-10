use std::io::{self, Read, Seek, SeekFrom};

use super::gdfx;
use super::io::{RangeRef, ReadFromRange};

#[derive(Debug, Clone, Copy)]
pub enum Iso {
    Xgd3,
    Xgd2,
    Xgd1,
    Xsf,
}

impl Iso {
    pub const ALL_SUPPORTED: &[Iso] = &[Iso::Xgd3, Iso::Xgd2, Iso::Xgd1, Iso::Xsf];

    pub fn data_volume_offset(self) -> u64 {
        match self {
            Iso::Xgd3 => 0x2080000,
            Iso::Xgd2 => 0xfd90000,
            Iso::Xgd1 => 0x18300000,
            Iso::Xsf => 0,
        }
    }

    pub fn gdfx_volume(&self, rr: RangeRef<Self>) -> RangeRef<gdfx::FileSystem> {
        rr.slice(self.data_volume_offset()..)
    }
}

impl ReadFromRange for Iso {
    fn read_from_range<R: Read + Seek>(mut r: R, off: u64, _len: u64) -> io::Result<Self> {
        use std::io::{Error, ErrorKind::*};

        let mut magic = [0u8; gdfx::MAGIC_BYTES.len()];

        for iso in Iso::ALL_SUPPORTED {
            let magic_offset = iso.data_volume_offset() + gdfx::DESCRIPTOR_OFFSET;

            match r
                .seek(SeekFrom::Start(off + magic_offset))
                .and_then(|_| r.read_exact(&mut magic))
            {
                Ok(_) if &magic == gdfx::MAGIC_BYTES => return Ok(*iso),
                Ok(_) => continue,
                Err(e) if e.kind() == UnexpectedEof => continue,
                Err(e) => return Err(e),
            }
        }

        Err(Error::new(InvalidData, "GDFX magic bytes not found"))
    }
}
