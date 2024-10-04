use anyhow::{Context, Error};
use clap::{command, Parser};

use iso2god::game_list;

mod unity;

#[derive(Clone, clap::ValueEnum)]
enum Source {
    BuiltIn,
    Unity,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(color = clap::ColorChoice::Never)]
struct Cli {
    #[arg(value_enum)]
    source: Source,

    title_id: String,
}

fn main() -> Result<(), Error> {
    let args = Cli::parse();

    let title_id = u32::from_str_radix("4D530064", 16)?;

    match args.source {
        Source::BuiltIn => {
            println!("querying the built-in DB for title ID {}", args.title_id);

            if let Some(name) = game_list::find_title_by_id(title_id) {
                let title_id = format!("{:08X}", title_id);
                println!("Title ID: {title_id}");
                println!("    Name: {name}");
            } else {
                println!("title not found in built-in database");
            }
        }

        Source::Unity => {
            println!("querying XboxUnity for title ID {}", args.title_id);

            let client = unity::Client::new().context("error creating XboxUnity client")?;

            let unity_title_info = client
                .find_xbox_360_title_id(title_id)
                .context("error querying XboxUnity")?;

            if let Some(unity_title_info) = &unity_title_info {
                println!("{unity_title_info}");
            } else {
                println!("no XboxUnity title info available");
            }
        }
    }

    Ok(())
}
