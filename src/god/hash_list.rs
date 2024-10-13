use std::io::{Read, Write};

use sha1::{Digest, Sha1};

use anyhow::Error;

pub struct HashList {
    buffer: [u8; 4096],
    len: usize,
}

impl HashList {
    pub fn bytes(&self) -> &[u8; 4096] {
        &self.buffer
    }

    pub fn new() -> HashList {
        HashList {
            buffer: [0u8; 4096],
            len: 0,
        }
    }

    pub fn read<R: Read>(mut reader: R) -> Result<HashList, Error> {
        let mut buffer = [0u8; 4096];
        reader.read_exact(&mut buffer)?;

        let len = buffer
            .chunks(20)
            .position(|c| *c == [0u8; 20])
            .map(|p| p * 20)
            .unwrap_or(buffer.len());

        Ok(HashList { buffer, len })
    }

    pub fn add_hash(&mut self, hash: &[u8; 20]) {
        self.buffer[self.len..self.len + 20].copy_from_slice(hash);
        self.len += 20;
    }

    pub fn add_block_hash(&mut self, block: &[u8]) {
        self.add_hash(&Sha1::digest(block).into())
    }

    pub fn digest(&self) -> [u8; 20] {
        Sha1::digest(&self.buffer).into()
    }

    pub fn write<W: Write>(&self, mut writer: W) -> Result<(), Error> {
        writer.write_all(&self.buffer)?;
        Ok(())
    }
}
