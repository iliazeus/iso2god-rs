//! https://free60.org/System-Software/Systems/GDFX/

use bitflags::bitflags;
use byteorder::{ReadBytesExt, LE};
use std::io::{Read, Seek, SeekFrom};
use std::ops::Range;

use super::{ReadFromSlice, ReadSlice, SECTOR_SIZE};

#[derive(Debug, Clone, Copy)]
pub struct Extent {
    /// first sector index
    pub sector: u32,

    /// size in bytes
    pub size: u32,
}

impl Extent {
    pub fn sectors(self) -> Range<u32> {
        let start = self.sector;
        let end = self.sector + self.size.div_ceil(SECTOR_SIZE as u32);
        start..end
    }

    pub fn bytes(self) -> Range<u64> {
        let start = self.sector as u64 * SECTOR_SIZE;
        let end = start + self.size as u64;
        start..end
    }
}

#[derive(Debug, Clone)]
pub struct FileSystem {
    pub root_dir: Extent,
    pub creation_time: u64,
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

pub type Dir = Vec<DirEntry>;

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
}

impl<R: Read + Seek> ReadFromSlice<R> for FileSystem {
    type Error = std::io::Error;
    fn read_from_slice(rs: &mut ReadSlice<R>) -> Result<Self, Self::Error> {
        let r = rs
            .by_ref()
            .slice(32 * SECTOR_SIZE..)
            .assert_magic(b"MICROSOFT*XBOX*MEDIA")?
            .into_inner();

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

impl<R: Read + Seek> ReadFromSlice<R> for Dir {
    type Error = std::io::Error;
    fn read_from_slice(rs: &mut ReadSlice<R>) -> Result<Self, Self::Error> {
        let mut entries = Vec::new();

        for off in rs.range().step_by(SECTOR_SIZE as usize) {
            let r = rs.by_ref().into_inner();
            r.seek(SeekFrom::Start(off))?;

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

        Ok(entries)
    }
}

pub fn walk<R: Read + Seek>(
    mut rs: ReadSlice<R>,
    dir: Extent,
    mut f: impl FnMut(&Vec<Vec<u8>>, DirEntry),
) -> Result<(), std::io::Error> {
    return rec(&mut rs, dir, &mut Vec::new(), &mut f);

    fn rec<R: Read + Seek>(
        rs: &mut ReadSlice<R>,
        extent: Extent,
        path: &mut Vec<Vec<u8>>,
        f: &mut impl FnMut(&Vec<Vec<u8>>, DirEntry),
    ) -> Result<(), std::io::Error> {
        let dir = rs.by_ref().slice(extent.bytes()).read::<Dir>()?;
        for entry in dir {
            let is_subdir = entry.attrs.contains(DirEntryAttrs::DIRECTORY);
            let data = entry.data;

            path.push(entry.name().to_owned());
            f(&path, entry);

            if is_subdir {
                rec(rs, data, path, f)?;
            }

            path.pop();
        }
        Ok(())
    }
}
