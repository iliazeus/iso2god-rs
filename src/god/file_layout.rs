use std::path::{Path, PathBuf};

use sha1::{Digest, Sha1};

use hex;

use crate::xex;

use super::*;

pub struct FileLayout<'a> {
    base_path: &'a Path,
    exe_info: &'a xex::XexExecutionInfo,
    content_type: ContentType,
}

impl<'a> FileLayout<'a> {
    pub fn new(
        base_path: &'a Path,
        exe_info: &'a xex::XexExecutionInfo,
        content_type: ContentType,
    ) -> FileLayout<'a> {
        FileLayout {
            base_path,
            exe_info,
            content_type,
        }
    }

    // TODO: why so complicated?
    fn get_unique_name(&self) -> String {
        let mut bytes = [0_u8; 10];

        bytes[0..4].copy_from_slice(&self.exe_info.title_id);
        bytes[4..8].copy_from_slice(&self.exe_info.media_id);
        bytes[8] = self.exe_info.disc_number;
        bytes[9] = self.exe_info.disc_count;

        let hash: [u8; 20] = Sha1::digest(bytes).into();

        hex::encode_upper(hash)
    }

    pub fn data_dir_path(&self) -> PathBuf {
        self.base_path
            .join(hex::encode_upper(self.exe_info.title_id))
            .join(format!("{:08X}", self.content_type as u32))
            .join(self.get_unique_name() + ".data")
    }

    pub fn part_file_path(&'a self, part_index: u64) -> PathBuf {
        self.data_dir_path().join(format!("Data{:04}", part_index))
    }

    pub fn con_header_file_path(&self) -> PathBuf {
        self.base_path
            .join(hex::encode_upper(self.exe_info.title_id))
            .join(format!("{:08X}", self.content_type as u32))
            .join(self.get_unique_name())
    }
}
