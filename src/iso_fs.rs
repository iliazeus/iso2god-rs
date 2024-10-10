use bitflags::bitflags;
use byteorder::{ReadBytesExt, LE};
use std::io::{Read, Seek, SeekFrom};

pub const SECTOR_SIZE: u64 = 2048;

#[derive(Debug, Clone, Copy)]
pub enum IsoType {
    Xgd3,
    Xgd2,
    Xgd1,
    Xsf,
}

impl IsoType {
    pub const SUPPORTED_TYPES: &[IsoType] =
        &[IsoType::Xgd3, IsoType::Xgd2, IsoType::Xgd1, IsoType::Xsf];

    pub fn gdfx_volume_offset(self) -> u64 {
        match self {
            IsoType::Xgd3 => 0x2080000,
            IsoType::Xgd2 => 0xfd90000,
            IsoType::Xgd1 => 0x18300000,
            IsoType::Xsf => 0,
        }
    }
}

/// https://free60.org/System-Software/Systems/GDFX/
#[derive(Debug)]
pub struct Fs<R> {
    reader: R,

    offset: u64,
    size: u64,

    root_dir_extent: FsExtent,

    #[allow(unused)]
    creation_time: u64,
}

impl<R> Fs<R> {
    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn size(&self) -> u64 {
        self.size
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FsExtent {
    // sector index
    pub sector: u32,

    // size in bytes
    pub size: u32,
}

impl FsExtent {
    pub fn advance_start(self) -> Self {
        if self.size == 0 {
            return self;
        }

        Self {
            sector: self.sector + 1,
            size: self.size.saturating_sub(SECTOR_SIZE as u32),
        }
    }

    pub fn is_empty(self) -> bool {
        self.size == 0
    }
}

impl<R: Read + Seek> Fs<R> {
    pub fn borrow_reader(&mut self) -> &mut R {
        &mut self.reader
    }

    pub fn seek_to_sector(&mut self, sector: u32) -> Result<&mut R, std::io::Error> {
        self.reader
            .seek(SeekFrom::Start(self.offset + (sector as u64) * SECTOR_SIZE))?;
        Ok(&mut self.reader)
    }

    pub fn seek_to_start(&mut self) -> Result<&mut R, std::io::Error> {
        self.reader.seek(SeekFrom::Start(self.offset))?;
        Ok(&mut self.reader)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FsTryReadError<R> {
    #[error("filesystem not found")]
    NotFound(R),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl<R: Read + Seek> Fs<R> {
    pub fn read_from_iso(reader: R) -> Result<Self, std::io::Error> {
        Self::try_read_from_iso(reader).map_err(|e| match e {
            FsTryReadError::NotFound(_) => std::io::Error::other("filesystem not found"),
            FsTryReadError::Io(e) => e,
        })
    }

    pub fn try_read_from_iso(mut reader: R) -> Result<Self, FsTryReadError<R>> {
        for iso_type in IsoType::SUPPORTED_TYPES.iter().copied() {
            match Self::try_read_from_offset(reader, iso_type.gdfx_volume_offset()) {
                Ok(fs) => return Ok(fs),
                Err(FsTryReadError::NotFound(r)) => reader = r,
                Err(e) => return Err(e),
            }
        }

        Err(FsTryReadError::NotFound(reader))
    }

    pub fn try_read_from_offset(mut reader: R, offset: u64) -> Result<Self, FsTryReadError<R>> {
        reader.seek(SeekFrom::Start(offset))?;

        match reader.seek_relative(32 * SECTOR_SIZE as i64) {
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(FsTryReadError::NotFound(reader))
            }
            Err(e) => return Err(e.into()),
            Ok(_) => {}
        }

        const EXPECTED_MAGIC: &[u8; 20] = b"MICROSOFT*XBOX*MEDIA";
        let mut actual_magic = [0u8; EXPECTED_MAGIC.len()];

        match reader.read_exact(&mut actual_magic) {
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(FsTryReadError::NotFound(reader))
            }
            Err(e) => return Err(e.into()),
            Ok(_) => {}
        }

        if &actual_magic != EXPECTED_MAGIC {
            return Err(FsTryReadError::NotFound(reader));
        }

        let root_dir_extent = FsExtent {
            sector: reader.read_u32::<LE>()?,
            size: reader.read_u32::<LE>()?,
        };

        let creation_time = reader.read_u64::<LE>()?;

        let end = reader.seek(SeekFrom::End(0))?;
        let size = end - offset;

        Ok(Self {
            reader,
            offset,
            size,
            root_dir_extent,
            creation_time,
        })
    }

    pub fn root_dir(&mut self) -> Dir {
        Dir {
            extent: self.root_dir_extent,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Dir {
    pub extent: FsExtent,
}

impl Dir {
    pub fn read_entries<'a, R: Read + Seek>(
        self,
        fs: &'a mut Fs<R>,
    ) -> Result<DirIter<'a, R>, std::io::Error> {
        DirIter::new(fs, self.extent)
    }
}

pub struct DirIter<'a, R> {
    fs: &'a mut Fs<R>,
    extent: FsExtent,
}

impl<'a, R: Read + Seek> Iterator for DirIter<'a, R> {
    type Item = Result<DirEntry, std::io::Error>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.try_next() {
            Ok(Some(x)) => Some(Ok(x)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

impl<'a, R: Read + Seek> DirIter<'a, R> {
    fn new(fs: &'a mut Fs<R>, extent: FsExtent) -> Result<Self, std::io::Error> {
        fs.seek_to_sector(extent.sector)?;
        Ok(Self { fs, extent })
    }

    fn try_next(&mut self) -> Result<Option<DirEntry>, std::io::Error> {
        while !self.extent.is_empty() {
            while let Some(entry) = DirEntry::try_read(&mut self.fs.reader)? {
                return Ok(Some(entry));
            }
            self.extent = self.extent.advance_start();
            self.fs.seek_to_sector(self.extent.sector)?;
        }
        Ok(None)
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct DirEntryAttrs: u8 {
        const ARCHIVE = 0x20;
        const DIRECTORY = 0x10;
        const HIDDEN = 0x02;
        const NORMAL = 0x80;
        const READ_ONLY = 0x01;
        const SYSTEM = 0x04;
    }
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub attrs: DirEntryAttrs,
    pub name_buf: [u8; 256],
    pub name_len: u8,
    pub data_extent: FsExtent,
    pub subtree_left: u16,
    pub subtree_right: u16,
}

impl DirEntry {
    pub fn name(&self) -> &[u8] {
        &self.name_buf[..(self.name_len as usize)]
    }

    pub fn as_dir(&self) -> Option<Dir> {
        if self.attrs.contains(DirEntryAttrs::DIRECTORY) {
            Some(Dir {
                extent: self.data_extent,
            })
        } else {
            None
        }
    }

    pub fn try_read(mut reader: impl Read + Seek) -> Result<Option<Self>, std::io::Error> {
        let subtree_left = reader.read_u16::<LE>()?;
        let subtree_right = reader.read_u16::<LE>()?;

        if subtree_left == 0xffff || subtree_right == 0xffff {
            return Ok(None);
        }

        let data_extent = FsExtent {
            sector: reader.read_u32::<LE>()?,
            size: reader.read_u32::<LE>()?,
        };

        let attrs = DirEntryAttrs::from_bits_truncate(reader.read_u8()?);

        let name_len = reader.read_u8()?;
        let mut name_buf = [0u8; 256];
        reader.read_exact(&mut name_buf[..(name_len as usize)])?;

        // TODO: is there a way to do this that does not require Seek?
        let alignment_mismatch = ((4 - reader.stream_position()? % 4) % 4) as i64;
        reader.seek_relative(alignment_mismatch)?;

        Ok(Some(Self {
            attrs,
            name_buf,
            name_len,
            data_extent,
            subtree_left,
            subtree_right,
        }))
    }
}
