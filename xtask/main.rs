use std::{
    env::{self, consts::EXE_SUFFIX},
    ffi::OsStr,
    fs::{self, File},
    io::Write as _,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use clap::Parser;

#[derive(Parser)]
#[command(about)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Wrapper of `cargo build`
    Build {
        /// Default value is "build".
        /// `cargo {command}`
        #[arg(long, default_value = "build")]
        command: String,
        /// Also build nrtm-installer
        #[arg(long)]
        dist: bool,
        options: Vec<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    eprintln!("Start xtask...");

    match args.command {
        Commands::Build {
            command,
            dist,
            options,
        } => build(command, dist, options)?,
    }

    eprintln!("Finish xtask.");

    Ok(())
}

fn build(
    build_command: String,
    dist: bool,
    build_options: Vec<String>,
) -> anyhow::Result<()> {
    eprintln!("Check nrtm package...");
    exec(
        Command::new("cargo")
            .arg("check")
            .args(["--package", "nrtm"]),
        false,
    )?;

    let build_target = get_build_target(&build_options).unwrap_or_default();
    if !build_target.is_empty() {
        exec(
            Command::new("rustup").args(["target", "add", &build_target]),
            false,
        )?;
    }

    let build_command = if env::var("CI").is_ok() && build_target.contains("-musl") {
        "zigbuild".into()
    } else {
        build_command
    };

    eprintln!("Compile nrtm package...");
    let executables = get_executables(
        Command::new("cargo")
            .arg(&build_command)
            .args(&build_options)
            .args(["--package", "nrtm"]),
    )?;

    let out_suffix = if build_target.is_empty() {
        "".into()
    } else {
        format!("-{build_target}")
    };
    let root_dir = get_workspace_root()?;
    let out_dir = root_dir.join(format!("out{out_suffix}"));
    let bin_dir = out_dir.join("bin");
    if !bin_dir.exists() {
        fs::create_dir_all(&bin_dir)?;
    }

    for executable in &executables {
        let copy_target = bin_dir.join(executable.file_name().unwrap());
        let copy_target = change_file_stem(&copy_target, "shim", "nvim");
        println!("Copy {} to {}", executable.display(), copy_target.display());
        fs::copy(executable, copy_target)?;
    }

    if !dist {
        return Ok(());
    }

    let zip_path = root_dir.join("out.zip");
    eprintln!("Create {}", zip_path.display());
    let file = File::create(zip_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::default();

    for entry in bin_dir.read_dir()? {
        let Ok(entry) = entry else {
            continue;
        };
        eprintln!("Add {} to the zip file...", entry.path().display());
        #[cfg(unix)]
        let options = {
            use std::os::unix::fs::PermissionsExt;
            let mode = entry.metadata()?.permissions().mode();
            options.unix_permissions(mode)
        };
        zip.start_file(entry.file_name().to_str().unwrap(), options)?;
        zip.write_all(&fs::read(entry.path())?)?;
    }

    eprintln!("Write the zip file...");
    zip.finish()?;

    eprintln!("Compile nrtm-installer package...");
    let executables = get_executables(
        Command::new("cargo")
            .arg(&build_command)
            .args(["--package", "nrtm-installer"]),
    )?;
    let executable = executables.get(0).unwrap();

    let copy_target = out_dir.join(format!("nrtm-installer{out_suffix}{EXE_SUFFIX}"));
    eprintln!("Copy {} to {}", executable.display(), copy_target.display());
    fs::copy(executable, copy_target)?;

    Ok(())
}

fn get_executables(command: &mut Command) -> anyhow::Result<Vec<PathBuf>> {
    let output = exec(command.args(["--message-format", "json"]), true)?;
    let r = output
        .lines()
        .filter_map(|json| {
            let Ok(data) = serde_json::from_str::<serde_json::Value>(json) else {
                return None;
            };
            let Some(serde_json::Value::String(s)) = data.get("executable") else {
                return None;
            };
            Some(PathBuf::from(s))
        })
        .collect();
    Ok(r)
}

fn get_build_target(build_options: &[String]) -> Option<String> {
    let Some(i) = build_options.iter().position(|e| e.starts_with("--target")) else {
        return None;
    };

    if build_options.get(i).unwrap() == "--target" {
        build_options.get(i + 1).map(String::from)
    } else {
        build_options
            .get(i)
            .unwrap()
            .strip_prefix("--target=")
            .map(String::from)
    }
}

fn get_workspace_root() -> anyhow::Result<PathBuf> {
    let json = exec(
        Command::new("cargo").args(["locate-project", "--workspace"]),
        true,
    )?;
    let data: serde_json::Value = serde_json::from_str(&json)?;
    let root_manifest = PathBuf::from(data.get("root").unwrap().as_str().unwrap());
    Ok(root_manifest.parent().unwrap().to_path_buf())
}

fn change_file_stem(path: &Path, from: &str, to: &str) -> PathBuf {
    let name = path.file_stem().unwrap();
    if OsStr::new(from) != name {
        return path.to_path_buf();
    }
    let mut new = path.to_path_buf();
    new.set_file_name(to);
    if let Some(ext) = path.extension() {
        new.set_extension(ext);
    }
    new
}

#[test]
fn change_file_stem_t() {
    let s = [
        ("shim.exe", "nvim.exe", ("shim", "nvim")),
        ("shim", "nvim", ("shim", "nvim")),
        ("hello.tar.gz", "hello.tar.gz", ("hello", "byebye")),
        (".foo.bar", ".foo.bar", ("foo", "bar")),
    ];

    for (before, after, (from, to)) in s {
        assert_eq!(
            PathBuf::from(after),
            change_file_stem(&PathBuf::from(before), from, to)
        );
    }
}

fn exec(command: &mut Command, read_stdout: bool) -> anyhow::Result<String> {
    if read_stdout {
        let output = String::from_utf8(
            command
                .stdin(Stdio::inherit())
                .stderr(Stdio::inherit())
                .output()?
                .stdout,
        )?;
        Ok(output)
    } else {
        command.spawn()?.wait()?;
        Ok("".into())
    }
}
