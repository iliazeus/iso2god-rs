use anyhow::Error;
use clap::Parser;
use std::fs::File;
use std::path::PathBuf;

use iso2god::new as iso2god;

#[derive(Parser)]
struct Args {
    /// ISO file
    source_iso: PathBuf,
}

fn main() -> Result<(), Error> {
    let args = Args::parse();

    let input = File::open(args.source_iso)?;

    let mut iso_input = iso2god::ReadSlice::from_whole(input)?;
    let iso = iso_input.read::<iso2god::Iso>()?;

    let mut fs_input = iso_input.slice(iso.data_volume_offset()..);
    let fs = fs_input.read::<iso2god::gdfx::Fs>()?;

    let root_dir = fs_input
        .by_ref()
        .slice(fs.root_dir.bytes())
        .read::<iso2god::gdfx::Dir>()?;

    if let Some(entry) = root_dir
        .into_iter()
        .find(|e| e.name().eq_ignore_ascii_case(b"default.xex"))
    {
        let mut xex_input = fs_input.by_ref().slice(entry.data.bytes());
        let xex = xex_input.read::<iso2god::Xex>()?;

        xex.fields.iter().for_each(|f| println!("{:08X}", f.key));

        if let Some(mut info_input) = xex.execution_id().map(|r| xex_input.slice(r)) {
            let info = info_input.read::<iso2god::xex::field::ExecutionId>()?;
            println!("{info:?}");
        }
    }

    let fs_offset = fs_input.range().start;

    iso2god::gdfx::walk(fs_input, fs.root_dir, |path, entry| {
        let path = path.join(&b'/');
        let path = String::from_utf8_lossy(&path);

        let abs_start = fs_offset + entry.data.bytes().start;
        let abs_end = fs_offset + entry.data.bytes().end;

        if (abs_start..abs_end).contains(&0x364eed05) {
            println!("/{}", path);
        }
    })?;

    Ok(())
}
