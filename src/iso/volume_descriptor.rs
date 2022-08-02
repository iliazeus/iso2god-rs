use byteorder::{ReadBytesExt, LE};

use std::io::{Read, Seek, SeekFrom};

use anyhow::{format_err, Error};

use super::iso_type::*;
use super::*;

#[derive(Debug)]
pub struct VolumeDescriptor {
    pub root_offset: u64,
    pub sector_size: u64,
    pub identifier: [u8; 20],
    pub root_directory_sector: u32,
    pub root_directory_size: u32,
    pub image_creation_time: [u8; 8],
    pub volume_size: u64,
    pub volume_sectors: u64,
}

impl VolumeDescriptor {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<VolumeDescriptor, Error> {
        let iso_type = IsoType::read(reader)?.ok_or_else(|| format_err!("invalid ISO format"))?;
        Self::read_of_type(reader, iso_type)
    }

    fn read_of_type<R: Read + Seek>(
        reader: &mut R,
        iso_type: IsoType,
    ) -> Result<VolumeDescriptor, Error> {
        reader.seek(SeekFrom::Start(0x20 * SECTOR_SIZE + iso_type.root_offset()))?;

        let mut identifier = [0_u8; 20];
        reader.read_exact(&mut identifier)?;

        let root_dir_sector = reader.read_u32::<LE>()?;
        let root_dir_size = reader.read_u32::<LE>()?;

        // TODO: more specific type?
        let mut image_creation_time = [0_u8; 8];
        reader.read_exact(&mut image_creation_time)?;

        let volume_size = reader.stream_len()? - iso_type.root_offset();
        let volume_sectors = volume_size / SECTOR_SIZE;

        Ok(VolumeDescriptor {
            sector_size: SECTOR_SIZE,
            root_offset: iso_type.root_offset(),
            identifier,
            root_directory_sector: root_dir_sector,
            root_directory_size: root_dir_size,
            image_creation_time,
            volume_size,
            volume_sectors,
        })
    }
}
