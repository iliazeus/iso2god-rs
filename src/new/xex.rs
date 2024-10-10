//! https://free60.org/System-Software/Formats/XEX/

use bitflags::bitflags;
use byteorder::{ReadBytesExt, BE};
use std::io::{self, Read, Seek, SeekFrom};

use super::io::{RangeRef, ReadFromRange};

#[derive(Debug, Clone)]
pub struct Xex {
    pub module_flags: ModuleFlags,
    pub code_offset: u32,
    pub cert_offset: u32,
    pub fields: Vec<FieldEntry>,
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
pub struct FieldEntry {
    pub key: u32,
    pub val: u32,
}

impl ReadFromRange for Xex {
    fn read_from_range<R: Read + Seek>(mut r: R, off: u64, _len: u64) -> io::Result<Self> {
        r.seek(SeekFrom::Start(off))?;

        const MAGIC: &[u8; 4] = b"XEX2";
        let mut magic = [0u8; MAGIC.len()];
        r.read_exact(&mut magic)?;
        if &magic != MAGIC {
            use std::io::{Error, ErrorKind::*};
            return Err(Error::new(InvalidData, "XEX magic bytes not found"));
        }

        let module_flags = ModuleFlags::from_bits_retain(r.read_u32::<BE>()?);
        let code_offset = r.read_u32::<BE>()?;
        r.read_u32::<BE>()?;
        let cert_offset = r.read_u32::<BE>()?;

        let field_count = r.read_u32::<BE>()? as usize;
        let mut fields = Vec::with_capacity(field_count);
        for _ in 0..field_count {
            fields.push(FieldEntry {
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

impl Xex {
    pub fn field<F: Field>(&self, xex_ref: RangeRef<Self>) -> Option<RangeRef<F>> {
        const { assert!(F::KEY & 0xFF > 0, "not implemented for 'immediate' fields") };

        self.fields.iter().find(|e| e.key == F::KEY).map(|e| {
            let off = e.val as u64;
            let len_words = (e.key & 0xFF) as u64;

            if len_words == 0xFF {
                xex_ref.slice(off..)
            } else {
                xex_ref.slice(off..off + 4 * len_words)
            }
        })
    }

    pub fn field_value<F: Field>(&self) -> Option<u32> {
        self.fields.iter().find(|e| e.key == F::KEY).map(|e| e.val)
    }
}

/// https://free60.org/System-Software/Formats/XEX/#header-ids
pub trait Field {
    const KEY: u32;
}

macro_rules! field_stub {
        ($(#[doc = $doc:literal])? $name:ident = $key:literal) => {
            $(#[doc = $doc])?
            #[derive(Debug, Clone)]
            pub struct $name;
            impl Field for $name {
                const KEY: u32 = $key;
            }
        };
    }

field_stub!(ResourceInfo = 0x2FF);
field_stub!(BaseFileFormat = 0x3FF);
field_stub!(BaseReference = 0x405);
field_stub!(DeltaPatchDescriptor = 0x5FF);
field_stub!(BoundingPath = 0x80FF);
field_stub!(DeviceId = 0x8105);
field_stub!(OriginalBaseAddress = 0x10001);
field_stub!(EntryPoint = 0x10100);
field_stub!(ImageBaseAddress = 0x10201);
field_stub!(ImportLibraries = 0x103FF);
field_stub!(ChecksumTimestamp = 0x18002);
field_stub!(EnabledForCallcap = 0x18102);
field_stub!(EnabledForFastcap = 0x18200);
field_stub!(OriginalPeName = 0x183FF);
field_stub!(StaticLibraries = 0x200FF);
field_stub!(TlsInfo = 0x20104);
field_stub!(DefaultStackSize = 0x20200);
field_stub!(DefaultFileSystemCacheSize = 0x20301);
field_stub!(DefaultHeapSize = 0x20401);
field_stub!(PageHeapSizeAndflags = 0x28002);
field_stub!(SystemFlags = 0x30000);
field_stub!(SerivceIdList = 0x401FF);
field_stub!(TitleWorkspaceSize = 0x40201);
field_stub!(GameRatings = 0x40310);
field_stub!(LanKey = 0x40404);
field_stub!(Xbox360Logo = 0x405FF);
field_stub!(MultidiscMediaIds = 0x406FF);
field_stub!(AlternateTitleIds = 0x407FF);
field_stub!(AdditionalTitleMemory = 0x40801);
field_stub!(ExportsByName = 0xE10402);

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

impl Field for ExecutionId {
    const KEY: u32 = 0x40006;
}

impl ReadFromRange for ExecutionId {
    fn read_from_range<R: Read + Seek>(mut r: R, off: u64, len: u64) -> io::Result<Self> {
        debug_assert_eq!(len, 4 * (0xff & Self::KEY as u64));
        r.seek(SeekFrom::Start(off))?;

        Ok(Self {
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
