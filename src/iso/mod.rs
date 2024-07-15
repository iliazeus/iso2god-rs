use std::io::{Read, Seek, SeekFrom};

use anyhow::Error;

pub mod directory_table;
pub mod iso_type;
pub mod volume_descriptor;

pub use directory_table::*;
pub use volume_descriptor::*;

pub const SECTOR_SIZE: u64 = 0x800;

pub struct IsoReader<R: Read + Seek> {
    pub volume_descriptor: VolumeDescriptor,
    pub directory_table: DirectoryTable,
    reader: R,
}

impl<R: Read + Seek> IsoReader<R> {
    pub fn read(mut reader: R) -> Result<IsoReader<R>, Error> {
        let volume_descriptor = VolumeDescriptor::read(&mut reader)?;
        let directory_table = DirectoryTable::read_root(&mut reader, &volume_descriptor)?;

        Ok(IsoReader {
            volume_descriptor,
            directory_table,
            reader,
        })
    }

    pub fn get_root(&mut self) -> Result<&mut R, Error> {
        self.reader
            .seek(SeekFrom::Start(self.volume_descriptor.root_offset))?;
        Ok(&mut self.reader)
    }

    pub fn get_entry(&mut self, path: &WindowsPath) -> Result<Option<&mut R>, Error> {
        let mut entry: Option<&DirectoryEntry> = None;
        let mut dir = Some(&self.directory_table);

        for name in path.components.iter() {
            entry = dir.and_then(|dir| dir.get_entry(name));
            dir = entry.and_then(|entry| entry.subdirectory.as_ref());
        }

        if let Some(entry) = entry {
            let position = self.volume_descriptor.root_offset
                + (entry.sector as u64) * self.volume_descriptor.sector_size;

            self.reader.seek(SeekFrom::Start(position))?;

            Ok(Some(&mut self.reader))
        } else {
            Ok(None)
        }
    }

    pub fn get_max_used_prefix_size(&self) -> u64 {
        return rec(&self.directory_table);
        fn rec(dir: &DirectoryTable) -> u64 {
            dir.entries
                .iter()
                .map(|entry| {
                    let mut v = (entry.sector as u64) * SECTOR_SIZE + (entry.size as u64);
                    if let Some(subdir) = &entry.subdirectory {
                        v = v.max(rec(subdir));
                    }
                    v
                })
                .max()
                .unwrap_or(0)
        }
    }
}

#[derive(Clone, Debug)]
pub struct WindowsPath {
    pub components: Vec<String>,
}

/// Case-insensitive (ascii case, for simplicity). Uses `\` as separator.
impl<'a, S: Into<&'a str>> From<S> for WindowsPath {
    fn from(path: S) -> WindowsPath {
        let path: &'a str = path.into();

        WindowsPath {
            components: path
                .split('\\')
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect(),
        }
    }
}
