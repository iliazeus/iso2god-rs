use anyhow::Error;
use clap::Parser;
use std::fs::File;
use std::path::PathBuf;

use iso2god::new::*;

#[derive(Parser)]
struct Args {
    /// ISO file
    source_iso: PathBuf,

    /// Output directory
    out_dir: PathBuf,
}

fn main() -> Result<(), Error> {
    let args = Args::parse();

    let input = File::open(args.source_iso)?;
    let mut input = ReadSlice::from_whole(input)?;

    let iso = input.read::<Iso>()?;

    let fs = input
        .narrow(iso.data_volume_offset()..)
        .read::<gdfx::FileSystem>()?;

    let mut files = Vec::new();
    gdfx::walk(input.by_ref(), fs.root_dir, |path, entry| {
        let path = path.join(&b'/');
        let path = String::from_utf8_lossy(&path).to_string();
        files.push((PathBuf::from(path), entry))
    })?;

    if std::fs::exists(&args.out_dir)? {
        std::fs::remove_dir_all(&args.out_dir)?;
    }
    std::fs::create_dir_all(&args.out_dir)?;

    for (path, entry) in files {
        let path = args.out_dir.join(path);
        println!("{}", path.to_str().unwrap());
        if entry.attrs.contains(gdfx::DirEntryAttrs::DIRECTORY) {
            std::fs::create_dir_all(path)?;
        } else {
            let mut src = input.by_ref().slice(entry.data.bytes()).take_whole()?;
            let mut dst = File::options()
                .create(true)
                .write(true)
                .truncate(true)
                .open(path)?;
            std::io::copy(&mut src, &mut dst)?;
        }
    }

    Ok(())
}
