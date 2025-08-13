use std::io::{Seek, SeekFrom, Write};

use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, Error};

use clap::{Parser, ValueEnum, arg, command};

use rayon::prelude::*;

use iso2god::executable::TitleInfo;
use iso2god::god::ContentType;
use iso2god::{game_list, god};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(color = clap::ColorChoice::Never)]
struct Cli {
    /// ISO file to convert
    source_iso: PathBuf,

    /// A folder to write resulting GOD files to
    dest_dir: PathBuf,

    /// Do not convert anything, just print the title info
    #[arg(long)]
    dry_run: bool,

    /// Set game title
    #[arg(long, value_name = "TITLE")]
    game_title: Option<String>,

    /// Whether to trim off unused space from the ISO image;
    /// passing no --trim flag at all is equivalent to "from-end"
    #[arg(
        verbatim_doc_comment,
        long,
        value_enum,
        require_equals = true,
        num_args = 0..=1,
        default_missing_value = "from-end"
    )]
    trim: Option<TrimMode>,

    /// Number of worker threads to use
    #[arg(long, short = 'j', value_name = "N", default_value_t = 1)]
    num_threads: usize,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default, ValueEnum)]
enum TrimMode {
    /// (default) Trim unallocated space from the end
    #[default]
    FromEnd,

    /// Trim nothing
    None,
    // TODO
    // FullRebuild,
}

fn main() -> Result<(), Error> {
    let args = Cli::parse();

    if args.num_threads == 1 {
        eprintln!(
            "The default number of threads was changed to 1 because of the problems witn Windows and/or hard drives."
        );
        eprintln!(
            "If you don't use Windows or use and SSD, might be worth increasing it with the -j <N> flag!"
        );
    }

    rayon::ThreadPoolBuilder::new()
        .num_threads(args.num_threads)
        .build_global()?;

    println!("extracting ISO metadata");

    let source_iso_file_meta =
        fs::metadata(&args.source_iso).context("error reading source ISO file metadata")?;

    let img = File::options().read(true).open(&args.source_iso)?;
    let xiso = std::io::BufReader::new(img);
    let mut xiso = xdvdfs::blockdev::OffsetWrapper::new(xiso).unwrap();
    
    let volume = xdvdfs::read::read_volume(&mut xiso).unwrap();

    let title_info =
        TitleInfo::from_image(&mut xiso, volume.clone()).context("error reading image executable")?;

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

    let root_offset = {
        // this is a workaround that leeks the offset implementation detail from xdvdfs which we need
        // to use the current god creation code which doesn't use the xdvdfs::blockdev::BlockDeviceRead trait
        xiso.seek(SeekFrom::Start(0))?;
        xiso.get_mut().stream_position().unwrap()
    };

    let data_size = if args.trim.unwrap_or_default() == TrimMode::FromEnd {
        volume.root_table.file_tree(&mut xiso)
            .context("error walking root directory tree")?
            .iter()
            .map(|dirent| {
                if dirent.1.node.dirent.data.is_empty() {
                    return 0;
                }
                return dirent.1.node.dirent.data.offset::<std::io::Error>(0).unwrap() + dirent.1.node.dirent.data.size() as u64
                })
            .max()
            .unwrap_or(0)
    } else {
        source_iso_file_meta.len() - root_offset
    };

    let block_count = data_size.div_ceil(god::BLOCK_SIZE);
    let part_count = block_count.div_ceil(god::BLOCKS_PER_PART);

    let file_layout = god::FileLayout::new(&args.dest_dir, &exe_info, content_type);

    println!("clearing data directory");

    ensure_empty_dir(&file_layout.data_dir_path()).context("error clearing data directory")?;

    println!("writing part files:  0/{part_count}");

    let progress = AtomicUsize::new(0);

    (0..part_count).into_par_iter().try_for_each(|part_index| {
        let mut iso_data_volume = File::open(&args.source_iso)?;
        iso_data_volume.seek(SeekFrom::Start(root_offset))?;

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
            last_part_size + (part_count - 1) * god::BLOCK_SIZE * 0xa290,
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
