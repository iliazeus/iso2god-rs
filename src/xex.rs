use std::io::{Read, Seek, SeekFrom};

use byteorder::{ReadBytesExt, BE};

use bitflags::bitflags;

use anyhow::{bail, Error};

#[derive(Clone, Debug)]
pub struct XexHeader {
    pub module_flags: XexModuleFlags,
    pub code_offset: u32,
    pub certificate_offset: u32,
    pub fields: XexHeaderFields,
}

bitflags! {
    pub struct XexModuleFlags: u32 {
        const DLL_MODULE = 0x08;
        const EXPORTS_TO_TITLE = 0x02;
        const MODULE_PATCH = 0x10;
        const SYSTEM_DEBUGGER = 0x04;
        const TITLE_MODULE = 0x01;

        // TODO: WTF: original code is ((Address & 1) == 0x40); isn't that impossible?
        const DELTA_PATCH = 0;

        // TODO: WTF: original code is ((Address & 1) == 0x20); isn't that impossible?
        const FULL_PATCH = 0;

        // TODO: WTF: original code is ((Address & 1) == 0x80); isn't that impossible?
        const USER_MODE = 0;
    }
}

#[derive(Clone, Default, Debug)]
pub struct XexHeaderFields {
    pub resource_info: Option<u32>,
    pub compression_info: Option<u32>,
    pub execution_info: Option<XexExecutionInfo>,
    pub base_file_format: Option<u32>,
    pub base_file_timestamp: Option<u32>,
    pub original_name: Option<u32>,
    pub ratings_info: Option<u32>,
    pub system_flags: Option<u32>,
}

mod sig {
    // some values repeat, just like in original code

    pub const BASE_FILE_FORMAT: u32 = 0x_00_00_03_ff;
    pub const BASE_FILE_TIMESTAMP: u32 = 0x_00_01_80_02;
    pub const COMPRESSION_INFO: u32 = 0x_00_00_03_ff;
    pub const EXECUTION_INFO: u32 = 0x_00_04_00_06;
    pub const MODULE_FLAGS: u32 = 0x_00_00_00_03;
    pub const ORIGINAL_NAME: u32 = 0x_00_01_83_ff;
    pub const RATINGS_INFO: u32 = 0x_00_04_03_10;
    pub const RESOURCE_INFO: u32 = 0x_00_00_02_ff;

    #[allow(dead_code)]
    pub const SYSTEM_FLAGS: u32 = 0x_00_00_00_03;
}

#[derive(Clone, Debug)]
pub struct XexExecutionInfo {
    pub media_id: [u8; 4],
    pub version: u32,
    pub base_version: u32,
    pub title_id: [u8; 4],
    pub platform: u8,
    pub executable_type: u8,
    pub disc_number: u8,
    pub disc_count: u8,
}

impl XexHeader {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<XexHeader, Error> {
        Self::check_magic_bytes(reader)?;
        Self::read_checked(reader)
    }

    fn check_magic_bytes<R: Read + Seek>(reader: &mut R) -> Result<(), Error> {
        let mut buf = [0_u8; 4];
        reader.read_exact(&mut buf)?;

        reader.seek(SeekFrom::Current(-4))?;

        if buf != "XEX2".as_bytes() {
            bail!("missing 'XEX2' magic bytes in XEX header");
        }

        Ok(())
    }

    fn read_checked<R: Read + Seek>(reader: &mut R) -> Result<XexHeader, Error> {
        let header_offset = reader.stream_position()?;

        let _ = reader.read_u32::<BE>()?;

        let module_flags = reader.read_u32::<BE>()?;
        let module_flags = XexModuleFlags::from_bits_truncate(module_flags);

        let code_offset = reader.read_u32::<BE>()?;

        let _ = reader.read_u32::<BE>()?;

        let certificate_offset = reader.read_u32::<BE>()?;

        let mut fields: XexHeaderFields = Default::default();
        let field_count = reader.read_u32::<BE>()?;

        for _ in 0..field_count {
            let key = reader.read_u32::<BE>()?;
            let value = reader.read_u32::<BE>()?;

            // some values repeat, just like in original code
            #[allow(unreachable_patterns)]
            match key {
                sig::RESOURCE_INFO => fields.resource_info = Some(value),
                sig::COMPRESSION_INFO => fields.compression_info = Some(value),

                sig::EXECUTION_INFO => {
                    let offset = reader.stream_position()?;
                    reader.seek(SeekFrom::Start(header_offset + (value as u64)))?;
                    fields.execution_info = Some(XexExecutionInfo::read(reader)?);
                    reader.seek(SeekFrom::Start(offset))?;
                }

                sig::BASE_FILE_FORMAT => fields.base_file_format = Some(value),
                sig::BASE_FILE_TIMESTAMP => fields.base_file_timestamp = Some(value),
                sig::ORIGINAL_NAME => fields.original_name = Some(value),
                sig::RATINGS_INFO => fields.ratings_info = Some(value),

                // sic! is this an oversight?
                sig::MODULE_FLAGS => fields.system_flags = Some(value),

                _ => {}
            };
        }

        Ok(XexHeader {
            module_flags,
            code_offset,
            certificate_offset,
            fields,
        })
    }
}

impl XexExecutionInfo {
    fn read<R: Read>(reader: &mut R) -> Result<XexExecutionInfo, Error> {
        let mut media_id = [0_u8; 4];
        reader.read_exact(&mut media_id)?;

        let version = reader.read_u32::<BE>()?;
        let base_version = reader.read_u32::<BE>()?;

        let mut title_id = [0_u8; 4];
        reader.read_exact(&mut title_id)?;

        let platform = reader.read_u8()?;
        let executable_type = reader.read_u8()?;
        let disc_number = reader.read_u8()?;
        let disc_count = reader.read_u8()?;

        Ok(XexExecutionInfo {
            media_id,
            version,
            base_version,
            title_id,
            platform,
            executable_type,
            disc_number,
            disc_count,
        })
    }
}
