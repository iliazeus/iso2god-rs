//! https://free60.org/System-Software/Formats/STFS/

use byteorder::{ReadBytesExt, BE};
use std::io::{Read, Seek};

use super::ReadFromSlice;

/// Not all fields might be meaningfully set.
#[derive(Debug, Clone)]
pub struct Metadata {
    pub media_id: u32,
    pub title_id: u32,
    pub disc_number: u8,
    pub disc_count: u8,
    pub display_name: LocalizedString,
    pub display_description: LocalizedString,
    pub publisher_name: String,
    pub title_name: String,
}

/// 18 locales
pub type LocalizedString = Vec<String>;

impl<R: Read + Seek> ReadFromSlice<R> for Metadata {
    type Error = std::io::Error;
    fn read_from_slice(rs: &mut super::ReadSlice<R>) -> Result<Self, Self::Error> {
        let r = rs.by_ref().seek_to_offset(0x0348)?;

        let _version = r.read_u32::<BE>()?;

        let _content_size = r.read_u64::<BE>()?;
        let media_id = r.read_u32::<BE>()?;
        let _update_version = r.read_u32::<BE>()?;
        let _update_base_version = r.read_u32::<BE>()?;
        let title_id = r.read_u32::<BE>()?;
        let _platform = r.read_u8()?;
        let _executable_type = r.read_u8()?;
        let disc_number = r.read_u8()?;
        let disc_count = r.read_u8()?;

        let mut r = rs.by_ref().seek_to_offset(0x0411)?;

        let display_name = read_localized_string(&mut r)?;
        let display_description = read_localized_string(&mut r)?;
        let publisher_name = read_utf16be::<64>(&mut r)?;
        let title_name = read_utf16be::<64>(&mut r)?;

        Ok(Metadata {
            media_id,
            title_id,
            disc_number,
            disc_count,
            display_name,
            display_description,
            publisher_name,
            title_name,
        })
    }
}

fn read_localized_string(mut r: impl Read) -> Result<LocalizedString, std::io::Error> {
    let mut entries = Vec::with_capacity(18);
    for _ in 0..18 {
        entries.push(read_utf16be::<64>(&mut r)?);
    }
    Ok(entries)
}

fn read_utf16be<const N: usize>(mut r: impl Read) -> Result<String, std::io::Error> {
    let mut buf = [0u16; N];
    r.read_u16_into::<BE>(&mut buf)?;
    let len = buf.iter().position(|x| *x == 0).unwrap_or(buf.len());
    let s = String::from_utf16_lossy(&buf[..len]);
    Ok(s)
}
