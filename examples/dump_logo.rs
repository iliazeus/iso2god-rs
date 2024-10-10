use clap::{command, Parser};
use iso2god::{iso_fs, xex};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, Write};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(color = clap::ColorChoice::Never)]
struct Cli {
    /// ISO file
    source_iso: PathBuf,

    out_file: PathBuf,
}

fn main() {
    let args = Cli::parse();

    println!("extracting ISO metadata");

    let source_iso_file =
        open_file_for_buffered_reading(&args.source_iso).expect("error opening source ISO file");

    let mut source_iso = iso_fs::Fs::read_from_iso(BufReader::new(source_iso_file))
        .expect("error reading source ISO");

    let root_dir_entries: Vec<_> = source_iso
        .root_dir()
        .read_entries(&mut source_iso)
        .and_then(|it| it.collect())
        .expect("error reading root dir");

    let default_xex = root_dir_entries
        .into_iter()
        .find(|e| e.name().eq_ignore_ascii_case(b"default.xex"))
        .expect("default.xex not found");

    let mut reader = source_iso
        .seek_to_sector(default_xex.data_extent.sector)
        .unwrap();

    let xex_header = xex::Header::read(&mut reader).expect("error reading default.xex header");

    let mut logo_reader = xex_header
        .field_reader(xex::FieldId::Xbox360Logo, reader)
        .expect("error reading logo")
        .expect("logo not found");

    let mut out_file =
        open_file_for_buffered_writing(&args.out_file).expect("error opening output file");

    std::io::copy(&mut logo_reader, &mut out_file).expect("error dumping logo");
}

fn open_file_for_buffered_reading(path: &Path) -> Result<impl Read + Seek, std::io::Error> {
    let file = File::options().read(true).open(path)?;
    let file = BufReader::with_capacity(8 * 1024 * 1024, file);
    Ok(file)
}

fn open_file_for_buffered_writing(path: &Path) -> Result<impl Write + Seek, std::io::Error> {
    let file = File::options().create(true).write(true).open(path)?;
    let file = BufWriter::with_capacity(8 * 1024 * 1024, file);
    Ok(file)
}
