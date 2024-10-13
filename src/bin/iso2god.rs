use std::io::{Seek, SeekFrom, Write};

use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, Error};

use clap::{arg, command, Parser};

use rayon::prelude::*;

use iso2god::executable::TitleInfo;
use iso2god::god::ContentType;
use iso2god::{game_list, god, iso};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(color = clap::ColorChoice::Never)]
struct Cli {
    /// ISO file to convert
    source_iso: PathBuf,

    /// A folder to write resulting GOD files to
    dest_dir: PathBuf,

    #[arg(long, hide = true)]
    #[deprecated(since = "1.5.0", note = "now uses a built-in database")]
    offline: bool,

    /// Do not convert anything, just print the title info
    #[arg(long)]
    dry_run: bool,

    /// Set game title
    #[arg(long)]
    game_title: Option<String>,

    /// Trim off unused space from the ISO image
    #[arg(long)]
    trim: bool,

    /// Number of worker threads to use
    #[arg(long, short = 'j')]
    num_threads: Option<usize>,
}

fn main() -> Result<(), Error> {
    let args = Cli::parse();

    #[allow(deprecated)]
    if args.offline {
        eprintln!("the --offline flag is deprecated: the tool now has a built-in title database, so it is always offline");
    }

    rayon::ThreadPoolBuilder::new()
        .num_threads(args.num_threads.unwrap_or(0))
        .build_global()?;

    println!("extracting ISO metadata");

    let source_iso_file = File::open(&args.source_iso).context("error opening source ISO file")?;

    let source_iso_file_meta =
        fs::metadata(&args.source_iso).context("error reading source ISO file metadata")?;

    let mut source_iso =
        iso::IsoReader::read(source_iso_file).context("error reading source ISO")?;

    let title_info =
        TitleInfo::from_image(&mut source_iso).context("error reading image executable")?;

    let exe_info = title_info.execution_info;
    let content_type = title_info.content_type;

    {
        let title_id = format!("{:08X}", exe_info.title_id);
        let name = game_list::find_title_by_id(exe_info.title_id).unwrap_or("(unknown)".to_owned());

        println!("Title ID: {title_id}");
        println!("    Name: {name}");
        match content_type {
            ContentType::GamesOnDemand => println!("    Type: Games on Demand"),
            ContentType::XboxOriginal => println!("    Type: Xbox Original"),
        }
    }

    if args.dry_run {
        return Ok(());
    }

    let data_size = if args.trim {
        source_iso.get_max_used_prefix_size()
    } else {
        let root_offset = source_iso.volume_descriptor.root_offset;
        source_iso_file_meta.len() - root_offset
    };

    let block_count = data_size.div_ceil(god::BLOCK_SIZE as u64);
    let part_count = block_count.div_ceil(god::BLOCKS_PER_PART);

    let file_layout = god::FileLayout::new(&args.dest_dir, &exe_info, content_type);

    println!("clearing data directory");

    ensure_empty_dir(&file_layout.data_dir_path()).context("error clearing data directory")?;

    println!("writing part files:  0/{part_count}");

    let progress = AtomicUsize::new(0);

    (0..part_count).into_par_iter().try_for_each(|part_index| {
        let mut iso_data_volume = File::open(&args.source_iso)?;
        iso_data_volume.seek(SeekFrom::Start(source_iso.volume_descriptor.root_offset))?;

        let part_file = file_layout.part_file_path(part_index);

        let part_file = File::options()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&part_file)
            .context("error creating part file")?;

        god::write_part(iso_data_volume, part_index, part_file)
            .context("error writing part file")?;

        let cur = 1 + progress.fetch_add(1, Ordering::Relaxed);
        println!("writing part files: {cur:2}/{part_count}");

        Ok::<_, anyhow::Error>(())
    })?;

    println!("calculating MHT hash chain");

    let mut mht =
        read_part_mht(&file_layout, part_count - 1).context("error reading part file MHT")?;

    for prev_part_index in (0..part_count - 1).rev() {
        let mut prev_mht =
            read_part_mht(&file_layout, prev_part_index).context("error reading part file MHT")?;

        prev_mht.add_hash(&mht.digest());

        write_part_mht(&file_layout, prev_part_index, &prev_mht)
            .context("error writing part file MHT")?;

        mht = prev_mht;
    }

    let last_part_size = fs::metadata(file_layout.part_file_path(part_count - 1))
        .map(|m| m.len())
        .context("error reading part file")?;

    println!("writing con header");

    let mut con_header = god::ConHeaderBuilder::new()
        .with_execution_info(&exe_info)
        .with_block_counts(block_count as u32, 0)
        .with_data_parts_info(
            part_count as u32,
            last_part_size + (part_count - 1) * (god::BLOCK_SIZE as u64) * 0xa290,
        )
        .with_content_type(content_type)
        .with_mht_hash(&mht.digest());

    let game_title = args
        .game_title
        .or(game_list::find_title_by_id(exe_info.title_id));
    if let Some(game_title) = game_title {
        con_header = con_header.with_game_title(&game_title);
    }

    let con_header = con_header.finalize();

    let mut con_header_file = File::options()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_layout.con_header_file_path())
        .context("cannot open con header file")?;

    con_header_file
        .write_all(&con_header)
        .context("error writing con header file")?;

    println!("done");

    Ok(())
}

fn ensure_empty_dir(path: &Path) -> Result<(), Error> {
    if fs::exists(path)? {
        fs::remove_dir_all(path)?;
    };
    fs::create_dir_all(path)?;
    Ok(())
}

fn read_part_mht(file_layout: &god::FileLayout, part_index: u64) -> Result<god::HashList, Error> {
    let part_file = file_layout.part_file_path(part_index);
    let mut part_file = File::options().read(true).open(part_file)?;
    god::HashList::read(&mut part_file)
}

fn write_part_mht(
    file_layout: &god::FileLayout,
    part_index: u64,
    mht: &god::HashList,
) -> Result<(), Error> {
    let part_file = file_layout.part_file_path(part_index);
    let mut part_file = File::options().write(true).open(part_file)?;
    mht.write(&mut part_file)?;
    Ok(())
}
