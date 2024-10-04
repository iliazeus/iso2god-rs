use std::io::{Cursor, Seek, SeekFrom, Write};

use byteorder::{WriteBytesExt, BE, LE};

use sha1::{Digest, Sha1};

use crate::executable::TitleExecutionInfo;

const EMPTY_LIVE: &[u8] = include_bytes!("empty_live.bin");

pub struct ConHeaderBuilder {
    buffer: Vec<u8>,
}

#[derive(Clone, Copy)]
pub enum ContentType {
    GamesOnDemand = 0x7000,
    XboxOriginal = 0x5000,
}

impl ConHeaderBuilder {
    pub fn new() -> Self {
        ConHeaderBuilder {
            buffer: Vec::from(EMPTY_LIVE),
        }
    }

    pub fn with_block_counts(mut self, blocks_allocated: u32, blocks_not_allocated: u16) -> Self {
        let mut cursor = Cursor::new(&mut self.buffer);

        cursor.seek(SeekFrom::Start(0x0392)).unwrap();
        cursor.write_u24::<BE>(blocks_allocated).unwrap();
        cursor.write_u16::<BE>(blocks_not_allocated).unwrap();

        self
    }

    pub fn with_content_type(mut self, content_type: ContentType) -> Self {
        let mut cursor = Cursor::new(&mut self.buffer);

        cursor.seek(SeekFrom::Start(0x0344)).unwrap();
        cursor.write_u32::<BE>(content_type as u32).unwrap();

        self
    }

    pub fn with_data_parts_info(mut self, part_count: u32, parts_total_size: u64) -> Self {
        let mut cursor = Cursor::new(&mut self.buffer);

        cursor.seek(SeekFrom::Start(0x03a0)).unwrap();
        cursor.write_u32::<LE>(part_count).unwrap(); // sic!

        cursor
            .write_u32::<BE>((parts_total_size / 0x0100) as u32)
            .unwrap();

        self
    }

    pub fn with_execution_info(mut self, exe_info: &TitleExecutionInfo) -> Self {
        let mut cursor = Cursor::new(&mut self.buffer);

        cursor.seek(SeekFrom::Start(0x0364)).unwrap();

        cursor.write_u8(exe_info.platform).unwrap();
        cursor.write_u8(exe_info.executable_type).unwrap();
        cursor.write_u8(exe_info.disc_number).unwrap();
        cursor.write_u8(exe_info.disc_count).unwrap();

        cursor.seek(SeekFrom::Start(0x0360)).unwrap();
        cursor.write_u32::<BE>(exe_info.title_id).unwrap();

        cursor.seek(SeekFrom::Start(0x0354)).unwrap();
        cursor.write_u32::<BE>(exe_info.media_id).unwrap();

        self
    }

    pub fn with_game_icon(mut self, png_bytes: Option<&[u8]>) -> Self {
        let empty_bytes = [0_u8; 20];
        let png_bytes = png_bytes.unwrap_or(&empty_bytes);

        let mut cursor = Cursor::new(&mut self.buffer);

        cursor.seek(SeekFrom::Start(0x1712)).unwrap();

        cursor.write_u32::<BE>(png_bytes.len() as u32).unwrap();
        cursor.write_u32::<BE>(png_bytes.len() as u32).unwrap(); // sic!

        cursor.seek(SeekFrom::Start(0x171a)).unwrap();
        cursor.write_all(png_bytes).unwrap();

        cursor.seek(SeekFrom::Start(0x571a)).unwrap();
        cursor.write_all(png_bytes).unwrap();

        self
    }

    pub fn with_game_title(mut self, game_title: &str) -> Self {
        let mut cursor = Cursor::new(&mut self.buffer);

        cursor.seek(SeekFrom::Start(0x0411)).unwrap();

        for code_unit in game_title.encode_utf16().into_iter() {
            cursor.write_u16::<BE>(code_unit).unwrap();
        }

        cursor.seek(SeekFrom::Start(0x1691)).unwrap();

        for code_unit in game_title.encode_utf16().into_iter() {
            cursor.write_u16::<BE>(code_unit).unwrap();
        }

        self
    }

    pub fn with_mht_hash(mut self, mht_hash: &[u8; 20]) -> Self {
        let mut cursor = Cursor::new(&mut self.buffer);

        cursor.seek(SeekFrom::Start(0x037d)).unwrap();
        cursor.write_all(mht_hash).unwrap();

        self
    }

    pub fn finalize(mut self) -> Vec<u8> {
        self.buffer[0x035b] = 0;
        self.buffer[0x035f] = 0;
        self.buffer[0x0391] = 0;

        {
            let digest: [u8; 20] = Sha1::digest(&self.buffer[0x0344..(0x0344 + 0xacbc)]).into();

            let mut cursor = Cursor::new(&mut self.buffer);
            cursor.seek(SeekFrom::Start(0x032c)).unwrap();

            cursor.write_all(&digest).unwrap();
        }

        self.buffer
    }
}
