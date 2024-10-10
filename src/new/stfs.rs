//! https://free60.org/System-Software/Formats/STFS/

use bitflags::bitflags;
use byteorder::{ReadBytesExt, BE};
use num_enum::TryFromPrimitive;
use std::io::{self, Read, Seek, SeekFrom};
use std::ops::Deref;

use super::ReadFromRange;

#[derive(Debug, Clone)]
pub struct Package {
    pub kind: PackageKind,
    pub content_type: ContentType,
    pub metadata_version: u32,
    pub content_size: u64,
    pub media_id: u32,
    pub version: u32,
    pub base_version: u32,
    pub title_id: u32,
    pub platform: Platform,
    pub executable_type: u8,
    pub disc_number: u8,
    pub disc_count: u8,
    pub save_game_id: u32,
    pub console_id: [u8; 5],
    pub profile_id: [u8; 8],

    pub file_system: FileSystem,
    pub data_file_count: u32,
    pub data_size: u64,

    // version 2 fields:
    pub series_id: [u8; 16],
    pub season_id: [u8; 16],
    pub season_number: u16,
    pub episode_number: u16,

    pub device_id: [u8; 20],
    pub display_name: LocalizedString,
    pub display_description: LocalizedString,
    pub publisher_name: SmallString,
    pub title_name: SmallString,
    pub transfer_flags: TransferFlags,
    pub thumbnail: Box<[u8]>,
    pub title_thumbnail: Box<[u8]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageKind {
    Con,
    Pirs,
    Live,
}

impl PackageKind {
    pub const ALL: &[Self] = &[Self::Con, Self::Pirs, Self::Live];

    pub const fn magic(self) -> &'static [u8; 4] {
        match self {
            Self::Con => b"CON ",
            Self::Pirs => b"PIRS",
            Self::Live => b"LIVE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(u32)]
pub enum ContentType {
    SavedGame = 0x0000001,
    MarketplaceContent = 0x0000002,
    Publisher = 0x0000003,
    Xbox360Title = 0x0001000,
    IptvPauseBuffer = 0x0002000,
    InstalledGame = 0x0004000,
    XboxOriginalGame = 0x0005000,
    // TODO: free60 has both XboxOriginalGame and XboxTitle share the same value
    // XboxTitle = 0x0005000,
    GameOnDemand = 0x0007000,
    AvatarItem = 0x0009000,
    Profile = 0x0010000,
    GamerPicture = 0x0020000,
    Theme = 0x0030000,
    CacheFile = 0x0040000,
    StorageDownload = 0x0050000,
    XboxSavedGame = 0x0060000,
    XboxDownload = 0x0070000,
    GameDemo = 0x0080000,
    Video = 0x0090000,
    GameTitle = 0x00A0000,
    Installer = 0x00B0000,
    GameTrailer = 0x00C0000,
    ArcadeTitle = 0x00D0000,
    Xna = 0x00E0000,
    LicenseStore = 0x00F0000,
    Movie = 0x0100000,
    Tv = 0x0200000,
    MusicVideo = 0x0300000,
    GameVideo = 0x0400000,
    PodcastVideo = 0x0500000,
    ViralVideo = 0x0600000,
    CommunityGame = 0x2000000,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
pub enum Platform {
    Unknown = 0,
    Xbox360 = 2,
    Pc = 4,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TransferFlags: u8 {
        const DEEP_LINK_SUPPORTED = 1 << 2;
        const DISABLE_NETWORK_STORAGE = 1 << 3;
        const KINECT_ENABLED = 1 << 4;
        const MOVE_ONLY_TRANSFER = 1 << 5;
        const DEVICE_ID_TRANSFER = 1 << 6;
        const PROFILE_ID_TRANSFER = 1 << 7;
    }
}

// A string under 64 UTF-16 code units in length
#[derive(Debug, Clone)]
pub struct SmallString(pub String);

impl SmallString {
    pub fn new() -> Self {
        Self(String::new())
    }
}

impl Deref for SmallString {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// A collection of strings under 64 UTF-16 code units in length
#[derive(Debug, Clone)]
pub struct LocalizedString(pub Vec<SmallString>);

impl LocalizedString {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

impl Deref for LocalizedString {
    type Target = [SmallString];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub enum FileSystem {
    Stfs {
        // TODO
    },
    Svod {
        block_cache_element_count: u8,
        worker_thread_processor: u8,
        worker_thread_priority: u8,
        digest: [u8; 20],
        device_features: u8,
        data_block_count: u32,  // u24
        data_block_offset: u32, // u24
    },
}

impl FileSystem {
    pub const fn magic(&self) -> u32 {
        match self {
            &Self::Stfs { .. } => 0,
            &Self::Svod { .. } => 1,
        }
    }
}

impl Package {
    pub fn new(kind: PackageKind, content_type: ContentType, file_system: FileSystem) -> Package {
        Package {
            kind,
            content_type,
            metadata_version: 0,
            content_size: 0,
            media_id: 0,
            version: 0,
            base_version: 0,
            title_id: 0,
            platform: Platform::Unknown,
            executable_type: 0,
            disc_number: 0,
            disc_count: 0,
            save_game_id: 0,
            console_id: [0; 5],
            profile_id: [0; 8],
            file_system,
            data_file_count: 0,
            data_size: 0,
            series_id: [0; 16],
            season_id: [0; 16],
            season_number: 0,
            episode_number: 0,
            device_id: [0; 20],
            display_name: LocalizedString::new(),
            display_description: LocalizedString::new(),
            publisher_name: SmallString::new(),
            title_name: SmallString::new(),
            transfer_flags: TransferFlags::empty(),
            thumbnail: Box::new([]),
            title_thumbnail: Box::new([]),
        }
    }
}

impl ReadFromRange for Package {
    fn read_from_range<R: Read + Seek>(mut r: R, off: u64, _len: u64) -> io::Result<Self> {
        use std::io::{Error, ErrorKind::*};

        r.seek(SeekFrom::Start(off))?;

        let mut magic = [0u8; 4];
        r.read_exact(&mut magic)?;

        let kind = PackageKind::ALL
            .iter()
            .copied()
            .find(|k| k.magic() == &magic)
            .ok_or_else(|| Error::new(InvalidData, "invalid package magic"))?;

        r.seek(SeekFrom::Start(off + 0x0344))?;

        let content_type: ContentType = r
            .read_u32::<BE>()?
            .try_into()
            .map_err(|e| Error::new(InvalidData, e))?;

        let metadata_version = r.read_u32::<BE>()?;
        let content_size = r.read_u64::<BE>()?;
        let media_id = r.read_u32::<BE>()?;
        let version = r.read_u32::<BE>()?;
        let base_version = r.read_u32::<BE>()?;
        let title_id = r.read_u32::<BE>()?;

        let platform: Platform = r
            .read_u8()?
            .try_into()
            .map_err(|e| Error::new(InvalidData, e))?;

        let executable_type = r.read_u8()?;
        let disc_number = r.read_u8()?;
        let disc_count = r.read_u8()?;
        let save_game_id = r.read_u32::<BE>()?;

        let mut console_id = [0u8; 5];
        r.read_exact(&mut console_id)?;

        let mut profile_id = [0u8; 8];
        r.read_exact(&mut profile_id)?;

        let desc_off = off + 0x244;
        let desc_len = 0x24;
        r.seek_relative(0x24)?;

        let data_file_count = r.read_u32::<BE>()?;
        let data_size = r.read_u64::<BE>()?;

        let file_system_type = r.read_u32::<BE>()?;
        let file_system = match file_system_type {
            0 => FileSystem::read_stfs(&mut r, desc_off, desc_len)?,
            1 => FileSystem::read_svod(&mut r, desc_off, desc_len)?,
            _ => {
                return Err(Error::new(
                    InvalidData,
                    "invalid file system descriptor type",
                ))
            }
        };

        let mut series_id = [0u8; 16];
        let mut season_id = [0u8; 16];
        let mut season_number = 0u16;
        let mut episode_number = 0u16;

        if metadata_version >= 1 {
            r.seek(SeekFrom::Start(off + 0x03b1))?;

            r.read_exact(&mut series_id)?;
            r.read_exact(&mut season_id)?;
            season_number = r.read_u16::<BE>()?;
            episode_number = r.read_u16::<BE>()?;
        }

        r.seek(SeekFrom::Start(off + 0x03fd))?;

        let mut device_id = [0u8; 20];
        r.read_exact(&mut device_id)?;

        let mut display_name = LocalizedString::read_from(&mut r)?;
        let mut display_description = LocalizedString::read_from(&mut r)?;
        let publisher_name = SmallString::read_from(&mut r)?;
        let title_name = SmallString::read_from(&mut r)?;

        let transfer_flags = r.read_u8().map(TransferFlags::from_bits_retain)?;

        r.seek(SeekFrom::Start(0x1712))?;

        let thumbnail_len = r.read_u32::<BE>()?;
        let title_thumbnail_len = r.read_u32::<BE>()?;

        let mut thumbnail = Vec::with_capacity(thumbnail_len as usize);
        r.by_ref()
            .take(thumbnail_len as u64)
            .read_to_end(&mut thumbnail)?;
        let thumbnail = thumbnail.into_boxed_slice();

        let mut title_thumbnail = Vec::with_capacity(title_thumbnail_len as usize);
        r.by_ref()
            .take(title_thumbnail_len as u64)
            .read_to_end(&mut title_thumbnail)?;
        let title_thumbnail = title_thumbnail.into_boxed_slice();

        if metadata_version >= 1 {
            r.seek(SeekFrom::Start(off + 0x541a))?;
            display_name.read_additional_from(&mut r)?;

            r.seek(SeekFrom::End(0x941a))?;
            display_description.read_additional_from(&mut r)?;
        }

        Ok(Self {
            kind,
            content_type,
            metadata_version,
            content_size,
            media_id,
            version,
            base_version,
            title_id,
            platform,
            executable_type,
            disc_number,
            disc_count,
            save_game_id,
            console_id,
            profile_id,
            file_system,
            data_file_count,
            data_size,
            series_id,
            season_id,
            season_number,
            episode_number,
            device_id,
            display_name,
            display_description,
            publisher_name,
            title_name,
            transfer_flags,
            thumbnail,
            title_thumbnail,
        })
    }
}

impl FileSystem {
    fn read_stfs<R: Read + Seek>(_r: R, _off: u64, _len: u64) -> Result<Self, io::Error> {
        Ok(Self::Stfs {})
    }

    fn read_svod<R: Read + Seek>(mut r: R, off: u64, _len: u64) -> Result<Self, io::Error> {
        use std::io::{Error, ErrorKind::*};

        r.seek(SeekFrom::Start(off))?;

        let size = r.read_u8()?;
        if size != 0x24 {
            return Err(Error::new(InvalidData, "wrong file system descriptor size"));
        }

        let block_cache_element_count = r.read_u8()?;
        let worker_thread_processor = r.read_u8()?;
        let worker_thread_priority = r.read_u8()?;

        let mut digest = [0u8; 20];
        r.read_exact(&mut digest)?;

        let device_features = r.read_u8()?;
        let data_block_count = r.read_u24::<BE>()?;
        let data_block_offset = r.read_u24::<BE>()?;

        Ok(Self::Svod {
            block_cache_element_count,
            worker_thread_processor,
            worker_thread_priority,
            digest,
            device_features,
            data_block_count,
            data_block_offset,
        })
    }
}

impl LocalizedString {
    pub fn read_from(mut r: impl Read) -> Result<Self, io::Error> {
        let mut v = Vec::with_capacity(18);
        for _ in 0..18 {
            let s = SmallString::read_from(&mut r)?;
            v.push(s);
        }
        Ok(Self(v))
    }

    pub fn read_additional_from(&mut self, mut r: impl Read) -> Result<(), io::Error> {
        self.0.reserve(6);
        for _ in 0..6 {
            let s = SmallString::read_from(&mut r)?;
            self.0.push(s);
        }
        Ok(())
    }
}

impl SmallString {
    pub fn read_from(r: impl Read) -> Result<Self, io::Error> {
        let mut buf = [0u16; 64];
        r.chain(io::repeat(0)).read_u16_into::<BE>(&mut buf)?;
        let len = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
        Ok(Self(String::from_utf16_lossy(&buf[..len])))
    }
}
