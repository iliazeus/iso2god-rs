use std::collections::VecDeque;
use std::io::{BufReader, Read, Seek};

use std::fs::File;
use std::path::{Path, PathBuf};

use anyhow::{Context, Error};

use clap::{command, Parser};

use iso2god::iso_fs;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(color = clap::ColorChoice::Never)]
struct Cli {
    /// ISO file
    source_iso: PathBuf,
}

fn main() -> Result<(), Error> {
    let args = Cli::parse();

    println!("extracting ISO metadata");

    let source_iso_file = open_file_for_buffered_reading(&args.source_iso)
        .context("error opening source ISO file")?;

    let mut source_iso = iso_fs::Fs::read_from_iso(BufReader::new(source_iso_file))
        .context("error reading source ISO")?;

    let mut queue = VecDeque::new();
    queue.push_back(("/".to_string(), source_iso.root_dir()));

    while let Some((base, dir)) = queue.pop_front() {
        for entry in dir.read_entries(&mut source_iso).unwrap() {
            let entry = entry.unwrap();

            let size = entry.data_extent.size;

            let mut path = base.clone();
            path += &String::from_utf8_lossy(entry.name());

            if let Some(subdir) = entry.as_dir() {
                path += "/";
                println!("{size:12} {path}");
                queue.push_back((path, subdir));
            } else {
                println!("{size:12} {path}");
            }
        }
    }

    Ok(())
}

fn open_file_for_buffered_reading(path: &Path) -> Result<impl Read + Seek, Error> {
    let file = File::options().read(true).open(path)?;
    let file = BufReader::with_capacity(8 * 1024 * 1024, file);
    Ok(file)
}
