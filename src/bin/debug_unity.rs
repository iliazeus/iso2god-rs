use anyhow::{Context, Error};

use clap::{command, Parser};

use hex;

use iso2god::unity;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(color = clap::ColorChoice::Never)]
struct Cli {
    title_id: String,
}

fn main() -> Result<(), Error> {
    let args = Cli::parse();

    println!("querying XboxUnity for title ID {}", args.title_id);

    let title_id: [u8; 4] = hex::decode(args.title_id)?
        .try_into()
        .map_err(|_| Error::msg("invalid title ID"))?;

    let client = unity::Client::new().context("error creating XboxUnity client")?;

    let unity_title_info = client
        .find_xbox_360_title_id(&title_id)
        .context("error querying XboxUnity")?;

    if let Some(unity_title_info) = &unity_title_info {
        println!("\n{}\n", unity_title_info);
    } else {
        println!("no XboxUnity title info available");
    }

    Ok(())
}
