use std::io::{Read, Seek};

use super::{ReadFromSlice, ReadSlice, SECTOR_SIZE};

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
}

impl<R: Read + Seek> ReadFromSlice<R> for Iso {
    type Error = std::io::Error;

    fn read_from_slice(rs: &mut ReadSlice<R>) -> Result<Self, Self::Error> {
        use std::io::{Error, ErrorKind::*};

        for iso in Iso::ALL_SUPPORTED {
            match rs
                .by_ref()
                .slice(iso.data_volume_offset()..)
                .slice(32 * SECTOR_SIZE..)
                .assert_magic(b"MICROSOFT*XBOX*MEDIA")
            {
                Ok(_) => return Ok(*iso),
                Err(e) if [UnexpectedEof, InvalidData].contains(&e.kind()) => continue,
                Err(e) => return Err(e),
            }
        }

        Err(Error::new(
            InvalidData,
            "data partition magic bytes not found",
        ))
    }
}
