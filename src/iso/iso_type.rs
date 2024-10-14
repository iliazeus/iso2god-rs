use std::io::{Read, Seek, SeekFrom};

use anyhow::Error;

use super::*;

#[derive(Debug)]
pub enum IsoType {
    Xgd3,
    Xgd2,
    Xgd1,
    Xsf,
}

impl IsoType {
    pub fn root_offset(&self) -> u64 {
        match self {
            IsoType::Xgd3 => 0x2080000,
            IsoType::Xgd2 => 0xfd90000,
            IsoType::Xgd1 => 0x18300000,
            IsoType::Xsf => 0,
        }
    }

    pub fn read<R: Read + Seek>(mut reader: R) -> Result<Option<IsoType>, Error> {
        if Self::check(&mut reader, IsoType::Xsf)? {
            return Ok(Some(IsoType::Xsf));
        }

        if Self::check(&mut reader, IsoType::Xgd2)? {
            return Ok(Some(IsoType::Xgd2));
        }

        if Self::check(&mut reader, IsoType::Xgd1)? {
            return Ok(Some(IsoType::Xgd1));
        }

        // original code had no extra check here, simply returning Xgd3 as fallback
        // https://github.com/eliecharra/iso2god-cli/blob/a3b266a5/Chilano/Xbox360/Iso/GDF.cs#L268

        if Self::check(&mut reader, IsoType::Xgd3)? {
            return Ok(Some(IsoType::Xgd3));
        }

        Ok(None)
    }

    fn check<R: Read + Seek>(mut reader: R, iso_type: IsoType) -> Result<bool, Error> {
        let mut buf = [0_u8; 20];
        match reader
            .seek(SeekFrom::Start(0x20 * SECTOR_SIZE + iso_type.root_offset()))
            .and_then(|_| reader.read_exact(&mut buf))
        {
            Ok(_) => Ok(&buf == b"MICROSOFT*XBOX*MEDIA"),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(false),
            Err(e) => Err(e.into()),
        }
    }
}
