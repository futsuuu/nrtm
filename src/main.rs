use std::env;

use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use tokio::{fs::File, io::AsyncWriteExt};
use tokio_stream::StreamExt;

/// A runtime manager for Neovim
#[derive(Parser)]
#[command(author, version, about)]
pub struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Download a release
    Get { version: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let release_name = format!(
        "nvim-{}",
        match env::consts::OS {
            "linux" => "linux64.tar.gz",
            "macos" => "macos.tar.gz",
            "windows" => "win64.zip",
            _ => anyhow::bail!("Unsupported OS"),
        }
    );

    match &args.command {
        Commands::Get { version } => {
            download_file(
                &Client::new(),
                &format!("https://github.com/neovim/neovim/releases/download/{version}/{release_name}"),
                &release_name
            ).await?;
        }
    }

    Ok(())
}

async fn download_file(client: &Client, url: &str, path: &str) -> anyhow::Result<()> {
    let res = client.get(url).send().await?;
    let total_size = res.content_length().unwrap() as u64;

    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("[{elapsed_precise}] [{wide_bar:.cyan/blue.dim}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
        .progress_chars("━╸╌"));

    let mut file = File::create(path).await?;
    let mut downloaded_size = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item?;
        file.write_all(&chunk).await?;
        downloaded_size += chunk.len();
        pb.set_position(downloaded_size as u64);
    }

    Ok(())
}
