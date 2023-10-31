use std::{
    env::consts::EXE_SUFFIX,
    fs::{self, File},
    io::{self, Write as _},
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use clap::Parser as _;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};

use nrtm::{github, shim, CACHE_DIR, NVIM_DIR};

/// A runtime manager for Neovim
#[derive(clap::Parser)]
#[command(author, version, about)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Restore Neovim version and NVIM_APPNAME
    #[command(name = "-")]
    Restore,
    /// Download a release
    Get { version: String },
    /// Remove the specified version
    Remove { version: String },
    /// Set version for use
    Use { version: String },
    /// Print all installed versions
    List,
    /// Print the path to an executable that used by shim
    Which,
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

    match &args.command {
        Commands::Restore => {
            shim::State::use_older_state()?;
        }
        Commands::Get { version } => {
            let releases = github::get_releases().await?;
            let asset = releases
                .iter()
                .filter_map(|release| {
                    if version == &release.tag_name {
                        println!("Release found: {}", release.html_url);
                        release.filter_assets()
                    } else {
                        None
                    }
                })
                .next();

            let Some(asset) = asset else {
                anyhow::bail!("Failed to get a asset.");
            };

            let asset_type = asset.get_type().unwrap();
            let download_target = CACHE_DIR.join(format!("{version}.{asset_type}"));
            download_file(
                &reqwest::Client::new(),
                &asset.browser_download_url,
                &download_target,
            )
            .await?;
            extract_archive(&download_target, &asset_type, &NVIM_DIR.join(version))?;

            eprintln!("Success to install Neovim {version}.");
        }
        Commands::Remove { version } => {
            fs::remove_dir_all(NVIM_DIR.join(version))?;
            eprintln!("Success to remove Neovim {version}.");
        }
        Commands::Use { version } => {
            let mut state = shim::State::read().unwrap_or_default();
            state.exe_path = if version == "system" {
                shim::State::default().exe_path
            } else {
                Some(
                    NVIM_DIR
                        .join(format!("{version}/bin/nvim{EXE_SUFFIX}"))
                        .display()
                        .to_string(),
                )
            };
            state.write()?;
        }
        Commands::List => {
            let exe_path = shim::State::read()
                .unwrap_or_default()
                .exe_path
                .map(PathBuf::from);

            for entry in NVIM_DIR.read_dir()? {
                let Ok(entry) = entry else {
                    continue;
                };
                let current_used = if let Some(ref exe_path) = exe_path {
                    exe_path.starts_with(entry.path())
                } else {
                    false
                };

                println!(
                    "{: <2}{}",
                    if current_used { "*" } else { "" },
                    entry.file_name().to_str().unwrap(),
                );
            }
        }
        Commands::Which => {
            println!(
                "{}",
                shim::State::read()
                    .unwrap_or_default()
                    .exe_path
                    .unwrap_or_default()
            );
        }
        Commands::App(args) => match &args.command {
            AppCommands::Use { name } => {
                let mut state = shim::State::read().unwrap_or_default();
                state.appname = Some(name.to_string());
                state.write()?;
            }
        },
        Commands::Update => {
            github::cache_response().await?;
            eprintln!("Success to update.");
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

    let mut file = File::create(path)?;
    let mut downloaded_size = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item?;
        file.write_all(&chunk)?;
        downloaded_size += chunk.len();
        pb.set_position(downloaded_size as u64);
    }

    Ok(())
}

fn extract_archive(
    archive: &PathBuf,
    archive_type: &github::AssetType,
    target: &PathBuf,
) -> anyhow::Result<()> {
    fs::create_dir_all(target)?;

    fn strip_toplevel(rel_path: impl AsRef<Path>) -> anyhow::Result<PathBuf> {
        let rel_path = rel_path.as_ref();
        let prefix = rel_path.iter().next().unwrap();
        let path = rel_path.strip_prefix(prefix)?.to_path_buf();
        Ok(path)
    }

    let archive = File::open(archive)?;

    match *archive_type {
        github::AssetType::Zip => {
            let mut archive = zip::ZipArchive::new(archive)?;

            for i in 0..archive.len() {
                let mut file = archive.by_index(i)?;
                let rel_path = strip_toplevel(file.mangled_name())?;
                let path = target.join(rel_path);

                if file.is_dir() {
                    fs::create_dir_all(&path)?;
                } else {
                    fs::create_dir_all(path.parent().unwrap())?;
                    let mut outfile = File::create(&path)?;
                    io::copy(&mut file, &mut outfile)?;

                    #[cfg(unix)]
                    if let Some(mode) = file.unix_mode() {
                        fs::set_permissions(&path, fs::Permissions::from_mode(mode))?;
                    }
                }
            }
        }
        github::AssetType::TarGz => {
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
        }
    }

    Ok(())
}
