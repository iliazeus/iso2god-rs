use anyhow::{Context, Error};
use clap::Parser;
use std::fs::File;
use std::path::PathBuf;

use iso2god::new::*;

#[derive(Parser)]
struct Args {
    /// ISO file
    source_iso: PathBuf,
}

fn main() -> Result<(), Error> {
    let args = Args::parse();

    let input = File::open(args.source_iso)?;
    let mut input = ReadSlice::from_whole(input)?;

    let iso = input.read::<Iso>()?;

    let fs = input
        .narrow(iso.data_volume_offset()..)
        .read::<gdfx::FileSystem>()?;

    let root_dir = input
        .by_ref()
        .slice(fs.root_dir.bytes())
        .read::<gdfx::Dir>()?;

    let nxeart = root_dir
        .into_iter()
        .find(|e| e.name().eq_ignore_ascii_case(b"nxeart"))
        .map(|e| input.narrow(e.data.bytes()).read::<stfs::Package>())
        .context("nxeart not found")??;

    println!("{nxeart:?}");

    Ok(())
}
