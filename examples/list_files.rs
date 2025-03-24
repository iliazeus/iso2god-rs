use std::fs::File;
use std::path::PathBuf;

use anyhow::{Context, Error};

use clap::{Parser, command};

use iso2god::iso;

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

    let source_iso_file = File::open(&args.source_iso).context("error opening source ISO file")?;

    let source_iso = iso::IsoReader::read(source_iso_file).context("error reading source ISO")?;

    println!("{:?}", source_iso.volume_descriptor);
    println!("max used size: {}", source_iso.get_max_used_prefix_size());

    print_dir(String::new(), &source_iso.directory_table);

    Ok(())
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
