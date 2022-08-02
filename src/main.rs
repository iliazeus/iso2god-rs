#![feature(fs_try_exists)]

use std::env;

use std::io::{BufReader, BufWriter, Read, Seek, Write};

use std::fs;
use std::fs::File;
use std::path::Path;

use num::integer::div_ceil;

use anyhow::Error;

use iso2god_rs::{god, iso, xex};

const USAGE: &'static str = "USAGE: iso2god <source_iso> <dest_dir>";

fn main() {
    let argv = env::args().collect::<Vec<_>>();

    let source_iso = argv.get(1).map(Path::new).expect(USAGE);
    let dest_dir = argv.get(2).map(Path::new).expect(USAGE);

    run(source_iso, dest_dir);
}

fn run(source_iso: &Path, dest_dir: &Path) {
    println!("extracting ISO metadata");

    let source_iso_file =
        open_file_for_buffered_reading(source_iso).expect("error opening source ISO file");

    let source_iso_file_meta =
        fs::metadata(source_iso).expect("error reading source ISO file metadata");

    let mut source_iso =
        iso::IsoReader::read(BufReader::new(source_iso_file)).expect("error reading source ISO");

    let mut default_xex = source_iso
        .get_entry(&"\\default.xex".into())
        .expect("error reading source ISO")
        .expect("default.xex file not found");

    let default_xex_header =
        xex::XexHeader::read(&mut default_xex).expect("error reading default.xex");

    println!("{:#?}", default_xex_header);

    // TODO: cropping

    let iso_file_size = source_iso_file_meta.len();
    let root_offset = source_iso.volume_descriptor.root_offset;

    let block_count = div_ceil(iso_file_size - root_offset, god::BLOCK_SIZE as u64);
    let part_count = div_ceil(block_count, god::BLOCKS_PER_PART);

    let exe_info = default_xex_header
        .fields
        .execution_info
        .expect("no execution info in default.xex header");

    // the original code does not seem to support other types
    let content_type = god::ContentType::GamesOnDemand;

    let file_layout = god::FileLayout::new(dest_dir, &exe_info, content_type);

    println!("clearing data directory");

    ensure_empty_dir(&file_layout.data_dir_path()).expect("error clearing data directory");

    let mut source_iso = source_iso.get_root().expect("error reading source iso");

    println!("writing part files");

    for part_index in 0..part_count {
        println!(".");

        let part_file = file_layout.part_file_path(part_index);

        let mut part_file =
            open_file_for_buffered_writing(&part_file).expect("error creating part file");

        god::write_part(&mut source_iso, &mut part_file).expect("error writing part file");
    }

    println!("calculating MHT hash chain");

    let mut mht = read_part_mht(&file_layout, part_count - 1).expect("error reading part file MHT");

    for prev_part_index in (0..part_count - 1).rev() {
        let mut prev_mht =
            read_part_mht(&file_layout, prev_part_index).expect("error reading part file MHT");

        prev_mht.add_hash(&mht.digest());

        write_part_mht(&file_layout, prev_part_index, &prev_mht)
            .expect("error writing part file MHT");

        mht = prev_mht;
    }

    let last_part_size = fs::metadata(file_layout.part_file_path(part_count - 1))
        .map(|m| m.len())
        .expect("error reading part file");

    println!("writing con header");

    let con_header = god::ConHeaderBuilder::new()
        .with_execution_info(&exe_info)
        .with_block_counts(block_count as u32, 0)
        .with_data_parts_info(
            part_count as u32,
            last_part_size + (part_count - 1) * (god::BLOCK_SIZE as u64) * 0xa290,
        )
        .with_content_type(god::ContentType::GamesOnDemand)
        .with_mht_hash(&mht.digest())
        .finalize();

    let mut con_header_file = open_file_for_buffered_writing(&file_layout.con_header_file_path())
        .expect("cannot open con header file");

    con_header_file
        .write_all(&con_header)
        .expect("error writing con header file");

    println!("done");
}

fn ensure_empty_dir(path: &Path) -> Result<(), Error> {
    if fs::try_exists(path)? {
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
