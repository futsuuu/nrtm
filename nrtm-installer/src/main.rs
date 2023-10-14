use std::{io::Cursor, path::Path};

use clap::Parser as _;

/// nrtm installer
#[derive(clap::Parser)]
#[command(author, version, about)]
struct Args {
    directory: String,
}

fn main() -> zip::result::ZipResult<()> {
    let args = Args::parse();
    let bin_dir = Path::new(&args.directory).join("bin");

    let zip_bytes = include_bytes!(concat!("../../out.zip"));
    let mut zip = zip::ZipArchive::new(Cursor::new(&zip_bytes[..]))?;
    zip.extract(bin_dir)?;

    Ok(())
}
