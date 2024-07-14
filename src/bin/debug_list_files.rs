use std::io::{BufReader, Read, Seek};

use std::fs::File;
use std::path::{Path, PathBuf};

use anyhow::{Context, Error};

use clap::{command, Parser};

use iso2god::iso;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(color = clap::ColorChoice::Never)]
struct Cli {
    /// Xbox 360 ISO file to convert
    source_iso: PathBuf,
}

fn main() -> Result<(), Error> {
    let args = Cli::parse();

    println!("extracting ISO metadata");

    let source_iso_file = open_file_for_buffered_reading(&args.source_iso)
        .context("error opening source ISO file")?;

    let source_iso = iso::IsoReader::read(BufReader::new(source_iso_file))
        .context("error reading source ISO")?;

    print_dir(String::new(), &source_iso.directory_table);

    Ok(())
}

fn open_file_for_buffered_reading(path: &Path) -> Result<impl Read + Seek, Error> {
    let file = File::options().read(true).open(path)?;
    let file = BufReader::with_capacity(8 * 1024 * 1024, file);
    Ok(file)
}

fn print_dir(path: String, dir: &iso::DirectoryTable) {
    for entry in dir.entries.iter() {
        if let Some(subdir) = &entry.subdirectory {
            print_dir(path.clone() + "/" + &entry.name, subdir);
        } else {
            println!("{:9} {}/{}", entry.size, path, entry.name);
        }
    }
}
