//! https://free60.org/System-Software/Systems/GDFX/

use bitflags::bitflags;
use byteorder::{ReadBytesExt, LE};
use std::{
    io::{self, Read, Seek, SeekFrom},
    ops::Range,
};

use super::io::{RangeRef, ReadFromRange};

pub const SECTOR_SIZE: u64 = 2048;
pub const DESCRIPTOR_OFFSET: u64 = 32 * SECTOR_SIZE;
pub const MAGIC_BYTES: &[u8; 20] = b"MICROSOFT*XBOX*MEDIA";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extent {
    /// first sector index
    pub sector: u32,

    /// size in bytes
    pub size: u32,
}

impl Extent {
    pub fn bytes(&self) -> Range<u64> {
        let start = SECTOR_SIZE * self.sector as u64;
        let end = start + self.size as u64;
        start..end
    }
}

#[derive(Debug, Clone)]
pub struct FileSystem {
    pub root_dir: Extent,
    pub creation_time: u64,
}

impl FileSystem {
    pub fn root_dir(&self, fs_ref: RangeRef<Self>) -> RangeRef<Dir> {
        fs_ref.slice(self.root_dir.bytes())
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

pub struct Dir {
    pub entries: Vec<DirEntry>,
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub attrs: DirEntryAttrs,
    pub name_buf: [u8; 256],
    pub name_len: u8,
    pub data: Extent,
    pub subtree_left: u16,
    pub subtree_right: u16,
}

impl DirEntry {
    pub fn name(&self) -> &[u8] {
        &self.name_buf[..(self.name_len as usize)]
    }

    pub fn as_file<T>(&self, fs_ref: RangeRef<FileSystem>) -> Option<RangeRef<T>> {
        if self.attrs.contains(DirEntryAttrs::DIRECTORY) {
            None
        } else {
            Some(fs_ref.slice(self.data.bytes()))
        }
    }

    pub fn as_dir(&self, fs_ref: RangeRef<FileSystem>) -> Option<RangeRef<Dir>> {
        if self.attrs.contains(DirEntryAttrs::DIRECTORY) {
            Some(fs_ref.slice(self.data.bytes()))
        } else {
            None
        }
    }
}

impl ReadFromRange for FileSystem {
    fn read_from_range<R: Read + Seek>(mut r: R, off: u64, _len: u64) -> io::Result<Self> {
        r.seek(SeekFrom::Start(off + DESCRIPTOR_OFFSET))?;

        let mut magic = [0u8; MAGIC_BYTES.len()];
        r.read_exact(&mut magic)?;
        if &magic != MAGIC_BYTES {
            use std::io::{Error, ErrorKind::*};
            return Err(Error::new(InvalidData, "GDFX magic bytes not found"));
        }

        let root_dir = Extent {
            sector: r.read_u32::<LE>()?,
            size: r.read_u32::<LE>()?,
        };
        let creation_time = r.read_u64::<LE>()?;

        Ok(Self {
            root_dir,
            creation_time,
        })
    }
}

impl ReadFromRange for Dir {
    fn read_from_range<R: Read + Seek>(mut r: R, off: u64, len: u64) -> io::Result<Self> {
        let mut entries = Vec::new();

        for sector_off in (off..off + len).step_by(SECTOR_SIZE as usize) {
            r.seek(SeekFrom::Start(sector_off))?;

            loop {
                let subtree_left = r.read_u16::<LE>()?;
                let subtree_right = r.read_u16::<LE>()?;

                if subtree_left == 0xffff || subtree_right == 0xffff {
                    break;
                }

                let data = Extent {
                    sector: r.read_u32::<LE>()?,
                    size: r.read_u32::<LE>()?,
                };

                let attrs = DirEntryAttrs::from_bits_truncate(r.read_u8()?);

                let name_len = r.read_u8()?;
                let mut name_buf = [0u8; 256];
                r.read_exact(&mut name_buf[..(name_len as usize)])?;

                // TODO: is there a way to do this that does not require Seek?
                let alignment_mismatch = ((4 - r.stream_position()? % 4) % 4) as i64;
                r.seek_relative(alignment_mismatch)?;

                entries.push(DirEntry {
                    attrs,
                    name_buf,
                    name_len,
                    data,
                    subtree_left,
                    subtree_right,
                })
            }
        }

        Ok(Self { entries })
    }
}

pub fn walk<R: Read + Seek>(
    mut r: R,
    fs_ref: RangeRef<FileSystem>,
    root_ref: RangeRef<Dir>,
    mut f: impl FnMut(&Vec<Vec<u8>>, DirEntry),
) -> io::Result<()> {
    return rec(&mut r, fs_ref, root_ref, &mut f, &mut Vec::new());

    fn rec<R: Read + Seek>(
        r: &mut R,
        fs_ref: RangeRef<FileSystem>,
        dir_ref: RangeRef<Dir>,
        f: &mut impl FnMut(&Vec<Vec<u8>>, DirEntry),
        path: &mut Vec<Vec<u8>>,
    ) -> io::Result<()> {
        for entry in dir_ref.read(r.by_ref())?.entries.into_iter() {
            let subdir = entry.as_dir(fs_ref);

            path.push(entry.name().to_owned());
            f(&path, entry);

            if let Some(subdir_ref) = subdir {
                rec(r, fs_ref, subdir_ref, f, path)?;
            }

            path.pop();
        }

        Ok(())
    }
}
