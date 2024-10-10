use anyhow::Error;
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

    let mut input = File::open(args.source_iso)?;

    let (iso_ref, iso) = Iso::read_whole(&mut input)?;

    let fs_ref = iso.gdfx_volume(iso_ref);
    let fs = fs_ref.read(&mut input)?;

    let root_dir_ref = fs.root_dir(fs_ref);
    let root_dir = root_dir_ref.read(&mut input)?;

    if let Some(xex_ref) = root_dir
        .entries
        .into_iter()
        .find(|e| e.name().eq_ignore_ascii_case(b"default.xex"))
        .and_then(|e| e.as_file::<Xex>(fs_ref))
    {
        let xex = xex_ref.read(&mut input)?;

        if let Some(exe_id_ref) = xex.field::<xex::ExecutionId>(xex_ref) {
            let exe_id = exe_id_ref.read(&mut input)?;
            println!("{exe_id:?}");
        }
    }

    gdfx::walk(&mut input, fs_ref, root_dir_ref, |path, entry| {
        let path = path.join(&b'/');
        let path = String::from_utf8_lossy(&path);
        let size = entry.data.size;
        println!("{size:12} /{path}");
    })?;

    Ok(())
}
