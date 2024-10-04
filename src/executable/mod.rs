use crate::god::ContentType;
use crate::iso::IsoReader;
use anyhow::{anyhow, Context, Error};
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
    pub fn from_xex<R: Read>(reader: &mut R) -> Result<TitleExecutionInfo, Error> {
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

    pub fn from_xbe<R: Read + Seek>(reader: &mut R) -> Result<TitleExecutionInfo, Error> {
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
        let content_type;
        let mut executable;

        match iso_image.get_entry(&"\\default.xex".into())? {
            Some(entry) => {
                executable = entry;
                content_type = ContentType::GamesOnDemand;
            }
            None => {
                executable = iso_image
                    .get_entry(&"\\default.xbe".into())?
                    .ok_or_else(|| anyhow!("no executable found in this image"))?;
                content_type = ContentType::XboxOriginal;
            }
        }

        let execution_info;

        match content_type {
            ContentType::GamesOnDemand => {
                let default_xex_header =
                    xex::XexHeader::read(&mut executable).context("error reading default.xex")?;

                execution_info = default_xex_header
                    .fields
                    .execution_info
                    .context("no execution info in default.xex header")?;
            }
            ContentType::XboxOriginal => {
                let default_xbe_header =
                    xbe::XbeHeader::read(&mut executable).context("error reading default.xbe")?;

                execution_info = default_xbe_header
                    .fields
                    .execution_info
                    .context("no execution info in default.xbe header")?;
            }
        }

        Ok(TitleInfo {
            content_type,
            execution_info,
        })
    }
}
