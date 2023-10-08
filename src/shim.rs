use std::{fs, path::PathBuf};

use once_cell::sync::Lazy;
use which::which_all_global;

use crate::{BIN_DIR, STATE_DIR};

static STATE_FILE: Lazy<PathBuf> = Lazy::new(|| STATE_DIR.join("shim"));
static SYSTEM_NVIM: Lazy<Option<PathBuf>> = Lazy::new(|| {
    let Ok(list) = which_all_global("nvim") else {
        return None;
    };

    for path in list {
        let Some(parent) = path.parent() else {
            continue;
        };
        if BIN_DIR.as_path() == parent {
            continue;
        }
        return Some(path);
    }

    None
});

pub struct State {
    pub exe_path: String,
    pub appname: String,
}

impl Default for State {
    fn default() -> Self {
        State {
            exe_path: match &*SYSTEM_NVIM {
                Some(path) => path.display().to_string(),
                None => "".into(),
            },
            appname: "".into(),
        }
    }
}

impl State {
    pub fn read() -> anyhow::Result<State> {
        if STATE_FILE.exists() {
            let content = fs::read_to_string(&*STATE_FILE)?;
            let (exe_path, appname) = content.split_once('\n').unwrap();
            let state = State {
                exe_path: exe_path.to_string(),
                appname: appname.to_string(),
            };
            Ok(state)
        } else {
            let state = Self::default();
            state.write()?;
            Ok(state)
        }
    }

    pub fn write(&self) -> anyhow::Result<()> {
        fs::write(&*STATE_FILE, format!("{}\n{}", self.exe_path, self.appname))?;
        Ok(())
    }
}
