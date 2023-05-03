use std::path::{Path, PathBuf};

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

    fn title_id_string(&self) -> String {
        hex::encode_upper(self.exe_info.title_id)
    }

    fn content_type_string(&self) -> String {
        format!("{:08X}", self.content_type as u32)
    }

    fn media_id_string(&self) -> String {
        hex::encode_upper(self.exe_info.media_id)
    }

    pub fn data_dir_path(&self) -> PathBuf {
        self.base_path
            .join(self.title_id_string())
            .join(self.content_type_string())
            .join(self.media_id_string() + ".data")
    }

    pub fn part_file_path(&'a self, part_index: u64) -> PathBuf {
        self.data_dir_path().join(format!("Data{:04}", part_index))
    }

    pub fn con_header_file_path(&self) -> PathBuf {
        self.base_path
            .join(self.title_id_string())
            .join(self.content_type_string())
            .join(self.media_id_string())
    }
}
