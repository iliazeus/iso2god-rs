//! https://free60.org/System-Software/Formats/XEX/

use bitflags::bitflags;
use byteorder::{ReadBytesExt, BE};
use std::io::{Read, Seek};
use std::ops::{Range, RangeFrom};

use super::{ReadFromSlice, ReadSlice};

#[derive(Debug, Clone)]
pub struct Xex {
    pub module_flags: ModuleFlags,
    pub code_offset: u32,
    pub cert_offset: u32,
    pub fields: Vec<Field>,
}

bitflags! {
    // based on https://free60.org/System-Software/Formats/XEX/#xex-header
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct ModuleFlags: u32 {
        const TITLE_MODULE = 0x01;
        const EXPORTS_TO_TITLE = 0x02;
        const SYSTEM_DEBUGGER = 0x04;
        const DLL_MODULE = 0x08;
        const MODULE_PATCH = 0x10;
        const FULL_PATCH = 0x20;
        const DELTA_PATCH = 0x40;
        const USER_MODE = 0x80;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Field {
    pub key: u32,
    pub val: u32,
}

impl Field {
    pub fn as_immediate(&self) -> Option<u32> {
        if self.key & 0xff == 0 {
            Some(self.val)
        } else {
            None
        }
    }

    pub fn as_range(&self) -> Option<Range<u64>> {
        let off = self.val as u64;
        let len = (self.key & 0xff) as u64;
        if len == 0 || len == 0xff {
            None
        } else {
            Some(off..(off + len * 4))
        }
    }

    pub fn as_range_from(&self) -> Option<RangeFrom<u64>> {
        let off = self.val as u64;
        let len = (self.key & 0xff) as u64;
        if len == 0xff {
            Some(off..)
        } else {
            None
        }
    }
}

/// https://free60.org/System-Software/Formats/XEX/#header-ids
pub mod field_key {
    pub const RESOURCE_INFO: u32 = 0x_00_00_02_ff;
    pub const BASE_FILE_FORMAT: u32 = 0x_00_00_03_ff;
    pub const BASE_REFERENCE: u32 = 0x_00_00_04_05;
    pub const DELTA_PATCH_DESCRIPTOR: u32 = 0x_00_00_05_ff;
    pub const BOUNDING_PATH: u32 = 0x_00_00_80_ff;
    pub const DEVICE_ID: u32 = 0x_00_00_81_05;
    pub const ORIGINAL_BASE_ADDRESS: u32 = 0x_00_01_00_01;
    pub const ENTRY_POINT: u32 = 0x_00_01_01_00;
    pub const IMAGE_BASE_ADDRESS: u32 = 0x_00_01_02_01;
    pub const IMPORT_LIBRARIES: u32 = 0x_00_01_03_ff;
    pub const CHECKSUM_TIMESTAMP: u32 = 0x_00_01_80_02;
    pub const ENABLED_FOR_CALLCAP: u32 = 0x_00_01_81_02;
    pub const ENABLED_FOR_FASTCAP: u32 = 0x_00_01_82_00;
    pub const ORIGINAL_PE_NAME: u32 = 0x_00_01_83_ff;
    pub const STATIC_LIBRARIES: u32 = 0x_00_02_00_ff;
    pub const TLS_INFO: u32 = 0x_00_02_01_04;
    pub const DEFAULT_STACK_SIZE: u32 = 0x_00_02_02_00;
    pub const DEFAULT_FILESYSTEM_CACHE_SIZE: u32 = 0x_00_02_03_01;
    pub const DEFAULT_HEAP_SIZE: u32 = 0x_00_02_04_01;
    pub const PAGE_HEAP_SIZE_AND_FLAGS: u32 = 0x_00_02_80_02;
    pub const SYSTEM_FLAGS: u32 = 0x_00_03_00_00;
    pub const EXECUTION_ID: u32 = 0x_00_04_00_06;
    pub const SERVICE_ID_LIST: u32 = 0x_00_04_01_ff;
    pub const TITLE_WORKSPACE_SIZE: u32 = 0x_00_04_02_01;
    pub const GAME_RATINGS: u32 = 0x_00_04_03_10;
    pub const LAN_KEY: u32 = 0x_00_04_04_04;
    pub const XBOX360_LOGO: u32 = 0x_00_04_05_ff;
    pub const MULTIDISC_MEDIA_IDS: u32 = 0x_00_04_06_ff;
    pub const ALTERNATE_TITLE_IDS: u32 = 0x_00_04_07_ff;
    pub const ADDITIONAL_TITLE_MEMORY: u32 = 0x_00_04_08_01;
    pub const EXPORTS_BY_NAME: u32 = 0x_00_e1_04_02;
}

impl Xex {
    pub fn execution_id(&self) -> Option<Range<u64>> {
        self.fields
            .iter()
            .find(|f| f.key == field_key::EXECUTION_ID)
            .and_then(|f| f.as_range())
    }
}

impl<R: Read + Seek> ReadFromSlice<R> for Xex {
    type Error = std::io::Error;
    fn read_from_slice(rs: &mut ReadSlice<R>) -> Result<Self, Self::Error> {
        let r = rs.by_ref().assert_magic(b"XEX2")?.into_inner();

        let module_flags = ModuleFlags::from_bits_retain(r.read_u32::<BE>()?);
        let code_offset = r.read_u32::<BE>()?;
        r.read_u32::<BE>()?;
        let cert_offset = r.read_u32::<BE>()?;

        let field_count = r.read_u32::<BE>()? as usize;
        let mut fields = Vec::with_capacity(field_count);
        for _ in 0..field_count {
            fields.push(Field {
                key: r.read_u32::<BE>()?,
                val: r.read_u32::<BE>()?,
            })
        }

        Ok(Self {
            module_flags,
            code_offset,
            cert_offset,
            fields,
        })
    }
}

pub mod field {
    use super::*;

    // TODO: better name? but i want it to match the field_key
    #[derive(Debug, Clone)]
    pub struct ExecutionId {
        pub media_id: u32,
        pub version: u32,
        pub base_version: u32,
        pub title_id: u32,
        pub platform: u8,
        pub executable_type: u8,
        pub disc_number: u8,
        pub disc_count: u8,
    }

    impl ExecutionId {
        pub const KEY: u32 = field_key::EXECUTION_ID;
    }

    impl<R: Read + Seek> ReadFromSlice<R> for ExecutionId {
        type Error = std::io::Error;
        fn read_from_slice(rs: &mut crate::new::ReadSlice<R>) -> Result<Self, Self::Error> {
            let r = rs.by_ref().seek_to_start()?;

            Ok(ExecutionId {
                media_id: r.read_u32::<BE>()?,
                version: r.read_u32::<BE>()?,
                base_version: r.read_u32::<BE>()?,
                title_id: r.read_u32::<BE>()?,
                platform: r.read_u8()?,
                executable_type: r.read_u8()?,
                disc_number: r.read_u8()?,
                disc_count: r.read_u8()?,
            })
        }
    }
}
