use crate::executable::TitleExecutionInfo;
use anyhow::{Error, bail};
use byteorder::{LE, ReadBytesExt};
use std::io::{Read, Seek, SeekFrom};

pub struct XbeHeader {
    // We only need these fields to get the cert address
    pub dw_base_addr: u32,
    pub dw_certificate_addr: u32,
    pub fields: XbeHeaderFields,
}

#[derive(Clone, Default, Debug)]
pub struct XbeHeaderFields {
    pub execution_info: Option<TitleExecutionInfo>,
}

impl XbeHeader {
    pub fn read<R: Read + Seek>(mut reader: R) -> Result<XbeHeader, Error> {
        Self::check_magic_bytes(&mut reader)?;

        // Offset 0x0104
        reader.seek(SeekFrom::Current(256))?;
        let dw_base_addr = reader.read_u32::<LE>()?;

        // Offset 0x0118
        reader.seek(SeekFrom::Current(16))?;
        let dw_certificate_addr = reader.read_u32::<LE>()?;

        let offset = reader.stream_position()? - 284;
        let cert_address = dw_certificate_addr - dw_base_addr;
        reader.seek(SeekFrom::Start(offset + (cert_address as u64)))?;

        Ok(XbeHeader {
            dw_base_addr,
            dw_certificate_addr,
            fields: XbeHeaderFields {
                execution_info: Some(TitleExecutionInfo::from_xbe(reader)?),
            },
        })
    }

    fn check_magic_bytes<R: Read + Seek>(mut reader: R) -> Result<(), Error> {
        let mut magic_bytes = [0u8; 4];
        reader.read_exact(&mut magic_bytes)?;

        if &magic_bytes != b"XBEH" {
            bail!("missing 'XBEH' magic bytes in XBE header");
        }

        Ok(())
    }
}
