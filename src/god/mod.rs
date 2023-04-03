use std::io::{Read, Seek, SeekFrom, Write};

use anyhow::Error;

mod con_header;
pub use con_header::*;

mod file_layout;
pub use file_layout::*;

mod gdf_sector;
pub use gdf_sector::*;

mod hash_list;
pub use hash_list::*;

pub const BLOCKS_PER_PART: u64 = 0xa1c4;
pub const BLOCKS_PER_SUBPART: u64 = 0xcc;
pub const BLOCK_SIZE: usize = 0x1000;
pub const FREE_SECTOR: u32 = 0x24;
pub const SUBPARTS_PER_PART: u32 = 0xcb;

pub fn write_part<R: Read, W: Write + Seek>(src: &mut R, dest: &mut W) -> Result<(), Error> {
    let mut block_buffer = Vec::<u8>::with_capacity(BLOCK_SIZE);
    let mut eof = false;

    let mut master_hash_list = HashList::new();

    let master_hash_list_position = dest.stream_position()?;
    master_hash_list.write(dest)?;

    for _subpart_index in 0..SUBPARTS_PER_PART {
        if eof {
            break;
        }

        let mut sub_hash_list = HashList::new();

        let sub_hash_list_position = dest.stream_position()?;
        sub_hash_list.write(dest)?;

        for _block_index in 0..BLOCKS_PER_SUBPART {
            src.by_ref()
                .take(BLOCK_SIZE as u64)
                .read_to_end(&mut block_buffer)?;

            if block_buffer.is_empty() {
                eof = true;
                break;
            }

            sub_hash_list.add_block_hash(&block_buffer);
            dest.write_all(&block_buffer)?;
            block_buffer.clear();
        }

        let next_position = dest.stream_position()?;

        dest.seek(SeekFrom::Start(sub_hash_list_position))?;
        sub_hash_list.write(dest)?;

        master_hash_list.add_block_hash(&sub_hash_list.to_bytes());

        dest.seek(SeekFrom::Start(next_position))?;
    }

    let next_position = dest.stream_position()?;

    dest.seek(SeekFrom::Start(master_hash_list_position))?;
    master_hash_list.write(dest)?;

    dest.seek(SeekFrom::Start(next_position))?;

    Ok(())
}
