use std::io::{Read, Seek, SeekFrom};

use anyhow::Error;

use super::*;

#[derive(Debug)]
pub enum IsoType {
    Gdf,
    Xgd3,
    Xsf,
}

impl IsoType {
    pub fn root_offset(&self) -> u64 {
        match self {
            IsoType::Gdf => 0xfd90000,
            IsoType::Xgd3 => 0x2080000,
            IsoType::Xsf => 0,
        }
    }

    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Option<IsoType>, Error> {
        if Self::check(reader, IsoType::Xsf)? {
            return Ok(Some(IsoType::Xsf));
        }

        if Self::check(reader, IsoType::Gdf)? {
            return Ok(Some(IsoType::Gdf));
        }

        // original code had no extra check here, simply returning Xgd3 as fallback
        // https://github.com/eliecharra/iso2god-cli/blob/a3b266a5/Chilano/Xbox360/Iso/GDF.cs#L268

        if Self::check(reader, IsoType::Xgd3)? {
            return Ok(Some(IsoType::Xgd3));
        }

        Ok(None)
    }

    fn check<R: Read + Seek>(reader: &mut R, iso_type: IsoType) -> Result<bool, Error> {
        let mut buf = [0_u8; 20];

        reader.seek(SeekFrom::Start(0x20 * SECTOR_SIZE + iso_type.root_offset()))?;
        reader.read_exact(&mut buf)?;

        Ok(buf == "MICROSOFT*XBOX*MEDIA".as_bytes())
    }
}
