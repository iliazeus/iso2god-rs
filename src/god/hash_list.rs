use std::io::{Read, Write};

use sha1::{Digest, Sha1};

use anyhow::Error;

pub struct HashList {
    buffer: Vec<u8>,
}

impl HashList {
    pub fn new() -> HashList {
        HashList {
            buffer: Vec::with_capacity(4096),
        }
    }

    pub fn read<R: Read>(reader: &mut R) -> Result<HashList, Error> {
        let mut reader = reader.by_ref().take(4096);
        let mut buffer = Vec::<u8>::with_capacity(4096);

        let mut block_buffer = Vec::<u8>::with_capacity(20);

        loop {
            reader.by_ref().take(20).read_to_end(&mut block_buffer)?;

            if block_buffer.is_empty() || block_buffer.iter().all(|x| *x == 0) {
                break;
            }

            buffer.append(&mut block_buffer);
        }

        Ok(HashList { buffer })
    }

    pub fn add_hash(&mut self, hash: &[u8; 20]) {
        self.buffer.extend_from_slice(hash);
    }

    pub fn add_block_hash(&mut self, block: &[u8]) {
        self.add_hash(&Sha1::digest(block).into())
    }

    pub fn digest(&self) -> [u8; 20] {
        Sha1::digest(self.to_bytes()).into()
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        writer.write_all(&self.to_bytes())?;
        Ok(())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = self.buffer.clone();
        buf.resize(4096, 0);
        buf
    }
}
