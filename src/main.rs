use std::{
    env::{self, consts::EXE_SUFFIX},
    path::{Path, PathBuf},
};

use clap::Parser as _;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::{fs::File, io::AsyncWriteExt};

use nrtm::{github, shim, APP_DIR, CACHE_DIR};

#[cfg(target_os = "windows")]
const ARCHIVE_EXT: &str = "zip";
#[cfg(not(target_os = "windows"))]
const ARCHIVE_EXT: &str = "tar.gz";

/// A runtime manager for Neovim
#[derive(clap::Parser)]
#[command(author, version, about)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Download a release
    Get { version: String },
    /// Set version for use
    Use { version: String },
    /// Manage NVIM_APPNAME
    App(AppArgs),
    /// Update cached response data
    Update,
}

#[derive(clap::Args)]
struct AppArgs {
    #[command(subcommand)]
    command: AppCommands,
}

#[derive(clap::Subcommand)]
enum AppCommands {
    /// Set NVIM_APPNAME
    Use { name: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let release_name = format!(
        "nvim-{}.{ARCHIVE_EXT}",
        match env::consts::OS {
            "linux" => "linux64",
            "macos" => "macos",
            "windows" => "win64",
            _ => anyhow::bail!("Unsupported OS"),
        }
    );

    match &args.command {
        Commands::Get { version } => {
            let download_target = CACHE_DIR.join(format!("{version}.{ARCHIVE_EXT}"));
            download_file(
                &reqwest::Client::new(),
                &format!("https://github.com/neovim/neovim/releases/download/{version}/{release_name}"),
                &download_target,
            ).await?;
            extract_archive(
                std::fs::File::open(download_target)?,
                APP_DIR.join(version),
            )?;
        }
        Commands::Use { version } => {
            let appname = if let Ok(state) = shim::State::new() {
                state.appname
            } else {
                String::new()
            };
            shim::State {
                exe_path: APP_DIR
                    .join(format!("{version}/bin/nvim{EXE_SUFFIX}"))
                    .display()
                    .to_string(),
                appname,
            }
            .write()?;
        }
        Commands::App(args) => match &args.command {
            AppCommands::Use { name } => {
                let exe_path = if let Ok(state) = shim::State::new() {
                    state.exe_path
                } else {
                    String::new()
                };
                shim::State {
                    appname: name.to_string(),
                    exe_path,
                }
                .write()?;
            }
        },
        Commands::Update => {
            github::update_request_cache().await?;
        }
    }

    Ok(())
}

async fn download_file(
    client: &reqwest::Client,
    url: &str,
    path: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let res = client.get(url).send().await?;
    let total_size = res.content_length().unwrap();

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

fn extract_archive(archive: std::fs::File, target: PathBuf) -> anyhow::Result<()> {
    std::fs::create_dir_all(&target)?;

    fn strip_toplevel(rel_path: impl AsRef<Path>) -> anyhow::Result<PathBuf> {
        let rel_path = rel_path.as_ref();
        let prefix = rel_path.iter().next().unwrap();
        let path = rel_path.strip_prefix(prefix)?.to_path_buf();
        Ok(path)
    }

    if ARCHIVE_EXT == "zip" {
        let mut archive = zip::ZipArchive::new(archive)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let rel_path = strip_toplevel(file.mangled_name())?;
            let path = target.join(rel_path);

            if file.is_dir() {
                std::fs::create_dir_all(&path)?;
            } else {
                std::fs::create_dir_all(path.parent().unwrap())?;
                let mut outfile = std::fs::File::create(&path)?;
                std::io::copy(&mut file, &mut outfile)?;

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Some(mode) = file.unix_mode() {
                        std::fs::set_permissions(
                            &path,
                            std::fs::Permissions::from_mode(mode),
                        )?;
                    }
                }
            }
        }

        return Ok(());
    }

    if ARCHIVE_EXT == "tar.gz" {
        let tar = flate2::read::GzDecoder::new(archive);
        let mut archive = tar::Archive::new(tar);

        for file in archive.entries()? {
            let Ok(mut file) = file else {
                continue;
            };

            let rel_path = strip_toplevel(file.path()?)?;
            let path = target.join(&rel_path);

            file.unpack(&path)?;
        }

        return Ok(());
    }

    Ok(())
}
