pub mod github;
pub mod shim;

use std::{env, fs, path::PathBuf};

use once_cell::sync::Lazy;

pub static BIN_DIR: Lazy<PathBuf> = Lazy::new(|| {
    env::current_exe()
        .expect("Failed to get the path for the current exe")
        .parent()
        .unwrap()
        .to_path_buf()
});
pub static BASE_DIR: Lazy<PathBuf> =
    Lazy::new(|| BIN_DIR.parent().unwrap().to_path_buf());
pub static APP_DIR: Lazy<PathBuf> = Lazy::new(|| init_dir("app"));
pub static CACHE_DIR: Lazy<PathBuf> = Lazy::new(|| init_dir("cache"));
pub static STATE_DIR: Lazy<PathBuf> = Lazy::new(|| init_dir("state"));

fn init_dir(name: &str) -> PathBuf {
    let path = BASE_DIR.join(name);
    fs::create_dir_all(&path).unwrap();
    path
}
