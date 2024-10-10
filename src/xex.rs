use bitflags::bitflags;
use byteorder::{ReadBytesExt, BE};
use std::io::{Read, Seek, SeekFrom};
use std::ops::Range;

bitflags! {
    // https://free60.org/System-Software/Formats/XEX/#xex-header
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    struct ModuleFlags: u32 {
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

pub struct Header {
    offset: u64,
    fields: Vec<(u32, Range<u64>)>,

    _module_flags: ModuleFlags,
    _code_offset: u32,
    _cert_offset: u32,
}

impl Header {
    pub fn read<R: Read + Seek>(mut reader: R) -> Result<Header, ReadError> {
        let offset = reader.stream_position()?;

        const EXPECTED_MAGIC: &[u8; 4] = b"XEX2";
        let mut actual_magic = [0u8; EXPECTED_MAGIC.len()];
        reader.read_exact(&mut actual_magic)?;
        if &actual_magic != EXPECTED_MAGIC {
            return Err(ReadError::MissingMagicBytes);
        }

        let _module_flags = ModuleFlags::from_bits_truncate(reader.read_u32::<BE>()?);
        let _code_offset = reader.read_u32::<BE>()?;
        let _cert_offset = reader.read_u32::<BE>()?;

        let field_count = reader.read_u32::<BE>()? as usize;
        let mut fields = Vec::with_capacity(field_count);
        for _ in 0..field_count {
            let key = reader.read_u32::<BE>()?;
            let offset = reader.read_u32::<BE>()? as u64;
            let length = (key & 0xff) as u64;
            fields.push((key, offset..(offset + length)));
        }

        Ok(Header {
            offset,
            _module_flags,
            _code_offset,
            _cert_offset,
            fields,
        })
    }

    pub fn field_reader<R: Read + Seek>(
        &self,
        key: FieldId,
        mut reader: R,
    ) -> Result<Option<std::io::Take<R>>, std::io::Error> {
        let field = self.fields.iter().find(|f| f.0 == key as u32);
        if let Some((_, range)) = field {
            reader.seek(SeekFrom::Start(self.offset + range.start))?;
            Ok(Some(reader.take(range.end - range.start)))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    #[error("missing 'XEX2' magic bytes")]
    MissingMagicBytes,

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

// https://free60.org/System-Software/Formats/XEX/#header-ids
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldId {
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
