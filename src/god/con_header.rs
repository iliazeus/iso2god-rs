use byteorder::{BE, ByteOrder, LE};

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

impl Default for ConHeaderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ConHeaderBuilder {
    pub fn new() -> Self {
        ConHeaderBuilder {
            buffer: Vec::from(EMPTY_LIVE),
        }
    }

    fn write_u8(&mut self, offset: usize, value: u8) {
        self.buffer[offset] = value;
    }

    fn write_u16_be(&mut self, offset: usize, value: u16) {
        BE::write_u16(&mut self.buffer[offset..], value);
    }

    fn write_u24_be(&mut self, offset: usize, value: u32) {
        BE::write_u24(&mut self.buffer[offset..], value);
    }

    fn write_u32_be(&mut self, offset: usize, value: u32) {
        BE::write_u32(&mut self.buffer[offset..], value);
    }

    fn write_u32_le(&mut self, offset: usize, value: u32) {
        LE::write_u32(&mut self.buffer[offset..], value);
    }

    fn write_bytes(&mut self, offset: usize, buf: &[u8]) {
        self.buffer[offset..offset + buf.len()].copy_from_slice(buf);
    }

    fn write_utf16_be(&mut self, offset: usize, s: &str) {
        for (i, c) in s.encode_utf16().chain([0]).enumerate() {
            self.write_u16_be(offset + i * 2, c);
        }
    }

    pub fn with_block_counts(mut self, blocks_allocated: u32, blocks_not_allocated: u16) -> Self {
        self.write_u24_be(0x0392, blocks_allocated);
        self.write_u16_be(0x0395, blocks_not_allocated);
        self
    }

    pub fn with_content_type(mut self, content_type: ContentType) -> Self {
        self.write_u32_be(0x0344, content_type as u32);
        self
    }

    pub fn with_data_parts_info(mut self, part_count: u32, parts_total_size: u64) -> Self {
        self.write_u32_le(0x03a0, part_count); // sic!
        self.write_u32_be(0x03a4, (parts_total_size / 0x0100) as u32);
        self
    }

    pub fn with_execution_info(mut self, exe_info: &TitleExecutionInfo) -> Self {
        // TODO: maybe just pick a suitable repr() for the struct, and write it whole?
        self.write_u32_be(0x0354, exe_info.media_id);
        self.write_u32_be(0x0360, exe_info.title_id);
        self.write_u8(0x0364, exe_info.platform);
        self.write_u8(0x0365, exe_info.executable_type);
        self.write_u8(0x0366, exe_info.disc_number);
        self.write_u8(0x0367, exe_info.disc_count);
        self
    }

    pub fn with_game_icon(mut self, png_bytes: Option<&[u8]>) -> Self {
        let png_bytes = png_bytes.unwrap_or(&[]);
        assert!(png_bytes.len() <= 0x0400);

        self.write_u32_be(0x1712, png_bytes.len() as u32);
        self.write_u32_be(0x1716, png_bytes.len() as u32);
        self.write_bytes(0x171a, png_bytes);
        self.write_bytes(0x571a, png_bytes);
        self
    }

    pub fn with_game_title(mut self, game_title: &str) -> Self {
        self.write_utf16_be(0x0411, game_title);
        self.write_utf16_be(0x1691, game_title);
        self
    }

    pub fn with_mht_hash(mut self, mht_hash: &[u8; 20]) -> Self {
        self.write_bytes(0x037d, mht_hash);
        self
    }

    pub fn finalize(mut self) -> Vec<u8> {
        self.buffer[0x035b] = 0;
        self.buffer[0x035f] = 0;
        self.buffer[0x0391] = 0;

        let digest: [u8; 20] = Sha1::digest(&self.buffer[0x0344..(0x0344 + 0xacbc)]).into();
        self.write_bytes(0x032c, &digest);

        self.buffer
    }
}
