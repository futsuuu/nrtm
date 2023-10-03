pub mod shim;

use std::{env, fs, path::PathBuf};

use once_cell::sync::Lazy;

pub static BASE_DIR: Lazy<PathBuf> = Lazy::new(|| match env::var("NRTM_DIR") {
    Ok(d) => PathBuf::from(d),
    Err(_) => home::home_dir()
        .expect("Failed to get home directory.")
        .join(".nrtm"),
});

pub static APP_DIR: Lazy<PathBuf> = Lazy::new(|| init_dir("app"));
pub static BIN_DIR: Lazy<PathBuf> = Lazy::new(|| init_dir("bin"));
pub static CACHE_DIR: Lazy<PathBuf> = Lazy::new(|| init_dir("cache"));
pub static STATE_DIR: Lazy<PathBuf> = Lazy::new(|| init_dir("state"));

fn init_dir(name: &str) -> PathBuf {
    let path = BASE_DIR.join(name);
    fs::create_dir_all(&path).unwrap();
    path
}
