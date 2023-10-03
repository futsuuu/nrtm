use std::{env::consts::EXE_SUFFIX, fs, path::PathBuf};

use crate::{BIN_DIR, STATE_DIR};

pub fn install() -> anyhow::Result<()> {
    let from = BIN_DIR.join(format!("shim{EXE_SUFFIX}"));
    let to = BIN_DIR.join(format!("nvim{EXE_SUFFIX}"));

    if from.exists() {
        // Rename `shim` to `nvim`
        fs::rename(from, to)?;
    }

    Ok(())
}

pub struct State {
    pub exe_path: String,
    pub appname: String,
}

impl State {
    pub fn new() -> anyhow::Result<State> {
        let content = fs::read_to_string(state_file())?;
        let (exe_path, appname) = content.split_once('\n').unwrap();
        Ok(State {
            exe_path: exe_path.to_string(),
            appname: appname.to_string(),
        })
    }

    pub fn write(&self) -> anyhow::Result<()> {
        fs::write(state_file(), format!("{}\n{}", self.exe_path, self.appname))?;
        Ok(())
    }
}

#[inline(always)]
fn state_file() -> PathBuf {
    STATE_DIR.join("shim")
}
