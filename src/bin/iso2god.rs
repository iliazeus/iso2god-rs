use std::io::{BufReader, BufWriter, Read, Seek, Write};

use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};

use num::integer::div_ceil;

use anyhow::{Context, Error};

use clap::{arg, command, Parser};

use iso2god::{game_list, god, iso, xex};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(color = clap::ColorChoice::Never)]
struct Cli {
    /// Xbox 360 ISO file to convert
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
}

fn main() -> Result<(), Error> {
    let args = Cli::parse();

    #[allow(deprecated)]
    if args.offline {
        eprintln!("the --offline flag is deprecated: the tool now has a built-in title database, so it is always offline");
    }

    println!("extracting ISO metadata");

    let source_iso_file = open_file_for_buffered_reading(&args.source_iso)
        .context("error opening source ISO file")?;

    let source_iso_file_meta =
        fs::metadata(&args.source_iso).context("error reading source ISO file metadata")?;

    let mut source_iso = iso::IsoReader::read(BufReader::new(source_iso_file))
        .context("error reading source ISO")?;

    let mut default_xex = source_iso
        .get_entry(&"\\default.xex".into())
        .context("error reading source ISO")?
        .context("default.xex file not found")?;

    let default_xex_header =
        xex::XexHeader::read(&mut default_xex).context("error reading default.xex")?;

    let exe_info = default_xex_header
        .fields
        .execution_info
        .context("no execution info in default.xex header")?;

    {
        let title_id = hex::encode_upper(exe_info.title_id);
        let name = game_list::find_title_by_id(exe_info.title_id).unwrap_or("(unknown)".to_owned());

        println!("Title ID: {title_id}");
        println!("    Name: {name}");
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

    let block_count = div_ceil(data_size, god::BLOCK_SIZE as u64);
    let part_count = div_ceil(block_count, god::BLOCKS_PER_PART);

    // the original code does not seem to support other types
    let content_type = god::ContentType::GamesOnDemand;

    let file_layout = god::FileLayout::new(&args.dest_dir, &exe_info, content_type);

    println!("clearing data directory");

    ensure_empty_dir(&file_layout.data_dir_path()).context("error clearing data directory")?;

    let mut source_iso = source_iso
        .get_root()
        .context("error reading source iso")?
        .take(data_size);

    println!("writing part files");

    for part_index in 0..part_count {
        println!("writing part {:2} of {:2}", part_index, part_count);

        let part_file = file_layout.part_file_path(part_index);

        let mut part_file =
            open_file_for_buffered_writing(&part_file).context("error creating part file")?;

        god::write_part(&mut source_iso, &mut part_file).context("error writing part file")?;
    }

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
        .with_content_type(god::ContentType::GamesOnDemand)
        .with_mht_hash(&mht.digest());

    let game_title = args
        .game_title
        .or(game_list::find_title_by_id(exe_info.title_id));
    if let Some(game_title) = game_title {
        con_header = con_header.with_game_title(&game_title);
    }

    let con_header = con_header.finalize();

    let mut con_header_file = open_file_for_buffered_writing(&file_layout.con_header_file_path())
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

fn open_file_for_buffered_writing(path: &Path) -> Result<impl Write + Seek, Error> {
    let file = File::options().create(true).write(true).open(path)?;
    let file = BufWriter::with_capacity(8 * 1024 * 1024, file);
    Ok(file)
}

fn open_file_for_buffered_reading(path: &Path) -> Result<impl Read + Seek, Error> {
    let file = File::options().read(true).open(path)?;
    let file = BufReader::with_capacity(8 * 1024 * 1024, file);
    Ok(file)
}
