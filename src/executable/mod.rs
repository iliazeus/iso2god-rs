use crate::god::ContentType;
use crate::iso::IsoReader;
use anyhow::{bail, Context, Error};
use byteorder::{ReadBytesExt, BE, LE};
use std::io::{Read, Seek, SeekFrom};

pub mod xbe;
pub mod xex;

#[derive(Clone, Debug)]
pub struct TitleExecutionInfo {
    pub media_id: u32,
    pub version: u32,
    pub base_version: u32,
    pub title_id: u32,
    pub platform: u8,
    pub executable_type: u8,
    pub disc_number: u8,
    pub disc_count: u8,
}

pub struct TitleInfo {
    pub content_type: ContentType,
    pub execution_info: TitleExecutionInfo,
}

impl TitleExecutionInfo {
    pub fn from_xex<R: Read>(mut reader: R) -> Result<TitleExecutionInfo, Error> {
        Ok(TitleExecutionInfo {
            media_id: reader.read_u32::<BE>()?,
            version: reader.read_u32::<BE>()?,
            base_version: reader.read_u32::<BE>()?,
            title_id: reader.read_u32::<BE>()?,
            platform: reader.read_u8()?,
            executable_type: reader.read_u8()?,
            disc_number: reader.read_u8()?,
            disc_count: reader.read_u8()?,
        })
    }

    pub fn from_xbe<R: Read + Seek>(mut reader: R) -> Result<TitleExecutionInfo, Error> {
        reader.seek(SeekFrom::Current(8))?;
        let title_id = reader.read_u32::<LE>()?;

        reader.seek(SeekFrom::Current(164))?;
        let version = reader.read_u32::<LE>()?;

        Ok(TitleExecutionInfo {
            media_id: 0,
            version,
            base_version: 0,
            title_id,
            platform: 0,
            executable_type: 0,
            disc_number: 1,
            disc_count: 1,
        })
    }
}

impl TitleInfo {
    pub fn from_image<R: Read + Seek>(iso_image: &mut IsoReader<R>) -> Result<TitleInfo, Error> {
        if let Some(mut executable) = iso_image.get_entry(&"\\default.xex".into())? {
            let default_xex_header =
                xex::XexHeader::read(&mut executable).context("error reading default.xex")?;
            let execution_info = default_xex_header
                .fields
                .execution_info
                .context("no execution info in default.xex header")?;

            Ok(TitleInfo {
                content_type: ContentType::GamesOnDemand,
                execution_info,
            })
        } else if let Some(mut executable) = iso_image.get_entry(&"\\default.xbe".into())? {
            let default_xbe_header =
                xbe::XbeHeader::read(&mut executable).context("error reading default.xbe")?;
            let execution_info = default_xbe_header
                .fields
                .execution_info
                .context("no execution info in default.xbe header")?;

            Ok(TitleInfo {
                content_type: ContentType::XboxOriginal,
                execution_info,
            })
        } else {
            bail!("no executable found in this image");
        }
    }
}
