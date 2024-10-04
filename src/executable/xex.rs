use std::io::{Read, Seek, SeekFrom};

use byteorder::{ReadBytesExt, BE};

use bitflags::bitflags;
use num_enum::TryFromPrimitive;

use crate::executable::TitleExecutionInfo;
use anyhow::{bail, Error};

#[derive(Clone, Debug)]
pub struct XexHeader {
    pub module_flags: XexModuleFlags,
    pub code_offset: u32,
    pub certificate_offset: u32,
    pub fields: XexHeaderFields,
}

bitflags! {
    // based on https://free60.org/System-Software/Formats/XEX/#xex-header
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct XexModuleFlags: u32 {
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

#[derive(Clone, Default, Debug)]
pub struct XexHeaderFields {
    pub execution_info: Option<TitleExecutionInfo>,
    // other fields will be added if and when necessary
}

// based on https://free60.org/System-Software/Formats/XEX/#header-ids
#[repr(u32)]
#[derive(Clone, Debug, PartialEq, Eq, TryFromPrimitive)]
#[allow(dead_code)]
enum XexHeaderFieldId {
    ResourceInfo = 0x_00_00_02_ff,
    BaseFileFormat = 0x_00_00_03_ff,
    BaseReference = 0x_00_00_04_05,
    DeltaPatchDescriptor = 0x_00_00_05_ff,
    BoundingPath = 0x_00_00_80_ff,
    DeviceId = 0x_00_00_81_05,
    OriginalBaseAddress = 0x_00_01_00_01,
    EntryPoint = 0x_00_01_01_00,
    ImageBaseAddress = 0x_00_01_02_01,
    ImportLibraries = 0x_00_01_03_ff,
    ChecksumTimestamp = 0x_00_01_80_02,
    EnabledForCallcap = 0x_00_01_81_02,
    EnabledForFastcap = 0x_00_01_82_00,
    OriginalPeName = 0x_00_01_83_ff,
    StaticLibraries = 0x_00_02_00_ff,
    TlsInfo = 0x_00_02_01_04,
    DefaultStackSize = 0x_00_02_02_00,
    DefaultFilesystemCacheSize = 0x_00_02_03_01,
    DefaultHeapSize = 0x_00_02_04_01,
    PageHeapSizeAndFlags = 0x_00_02_80_02,
    SystemFlags = 0x_00_03_00_00,
    ExecutionId = 0x_00_04_00_06,
    ServiceIdList = 0x_00_04_01_ff,
    TitleWorkspaceSize = 0x_00_04_02_01,
    GameRatings = 0x_00_04_03_10,
    LanKey = 0x_00_04_04_04,
    Xbox360Logo = 0x_00_04_05_ff,
    MultidiscMediaIds = 0x_00_04_06_ff,
    AlternateTitleIds = 0x_00_04_07_ff,
    AdditionalTitleMemory = 0x_00_04_08_01,
    ExportsByName = 0x_00_e1_04_02,
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

            let key = XexHeaderFieldId::try_from(key).ok();
            type Key = XexHeaderFieldId;

            match key {
                Some(Key::ExecutionId) => {
                    let offset = reader.stream_position()?;
                    reader.seek(SeekFrom::Start(header_offset + (value as u64)))?;
                    fields.execution_info = Some(TitleExecutionInfo::from_xex(reader)?);
                    reader.seek(SeekFrom::Start(offset))?;
                }

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
