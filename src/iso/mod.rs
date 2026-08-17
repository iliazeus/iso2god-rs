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
        // The data volume must contain the volume descriptor and the root
        // directory table, and neither appears as a directory entry - so
        // account for their extents explicitly. Subdirectory tables need no
        // special handling: their extents are their parent directories'
        // entries.
        let volume_descriptor_end = 0x21 * SECTOR_SIZE;
        let root_table_end = (self.volume_descriptor.root_directory_sector as u64) * SECTOR_SIZE
            + (self.volume_descriptor.root_directory_size as u64);

        return volume_descriptor_end
            .max(root_table_end)
            .max(rec(&self.directory_table));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Minimal base-0 image: volume descriptor at sector 0x20, one file
    /// entry in the root table. Table and file placement are parameters
    /// so tests can lay them out in either order.
    fn build_image(root_table_sector: u32, file_sector: u32, file_size: u32) -> Vec<u8> {
        let end_sector = root_table_sector.max(file_sector) as usize + 2;
        let mut img = vec![0_u8; end_sector * SECTOR_SIZE as usize];

        let desc = (0x20 * SECTOR_SIZE) as usize;
        img[desc..desc + 20].copy_from_slice(b"MICROSOFT*XBOX*MEDIA");
        img[desc + 20..desc + 24].copy_from_slice(&root_table_sector.to_le_bytes());
        img[desc + 24..desc + 28].copy_from_slice(&(SECTOR_SIZE as u32).to_le_bytes());

        let table = (root_table_sector as u64 * SECTOR_SIZE) as usize;
        img[table..table + 2].copy_from_slice(&0_u16.to_le_bytes());
        img[table + 2..table + 4].copy_from_slice(&0_u16.to_le_bytes());
        img[table + 4..table + 8].copy_from_slice(&file_sector.to_le_bytes());
        img[table + 8..table + 12].copy_from_slice(&file_size.to_le_bytes());
        img[table + 12] = 0x20; // ARCHIVE
        img[table + 13] = 1;
        img[table + 14] = b'a';
        for b in &mut img[table + 15..table + SECTOR_SIZE as usize] {
            *b = 0xff;
        }

        img
    }

    #[test]
    fn trim_includes_root_directory_table_placed_after_file_data() {
        // root table at sector 40, file data ending inside sector 33:
        // the trimmed volume must still reach the end of the root table.
        let iso = IsoReader::read(Cursor::new(build_image(40, 33, 100))).unwrap();
        assert!(iso.get_max_used_prefix_size() >= 41 * SECTOR_SIZE);
    }

    #[test]
    fn trim_is_governed_by_file_extents_when_tables_come_first() {
        // the common retail layout: table before the data. The last file
        // extent decides the size, exactly as before.
        let iso = IsoReader::read(Cursor::new(build_image(33, 40, 100))).unwrap();
        assert_eq!(iso.get_max_used_prefix_size(), 40 * SECTOR_SIZE + 100);
    }
}
