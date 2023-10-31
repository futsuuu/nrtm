use std::{fs, path::PathBuf};

use once_cell::sync::Lazy;
use which::which_all_global;

use crate::{BIN_DIR, STATE_DIR};

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

enum StateKind {
    Draft,
    Current,
    Old,
}

impl PartialEq for StateKind {
    fn eq(&self, rhs: &Self) -> bool {
        self.read().ok() == rhs.read().ok()
    }
}

impl StateKind {
    fn path(&self) -> PathBuf {
        use StateKind::*;
        match *self {
            Draft => STATE_DIR.join("draft.shim"),
            Current => STATE_DIR.join("current.shim"),
            Old => STATE_DIR.join("old.shim"),
        }
    }

    fn read(&self) -> anyhow::Result<Option<String>> {
        let path = self.path();
        if !path.exists() {
            return Ok(None);
        }
        let r = fs::read_to_string(path)?;
        Ok(Some(r))
    }

    fn write(&self, content: String) -> anyhow::Result<()> {
        fs::write(self.path(), content)?;
        Ok(())
    }

    fn replace_with(&self, kind: Self) -> anyhow::Result<()> {
        self.write(kind.read()?.unwrap_or_default())?;
        Ok(())
    }
}

#[derive(PartialEq)]
pub struct State {
    pub exe_path: Option<String>,
    pub appname: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        State {
            exe_path: SYSTEM_NVIM.clone().map(|p| p.display().to_string()),
            appname: None,
        }
    }
}

impl State {
    pub fn read() -> anyhow::Result<State> {
        let state = Self::_read(&StateKind::Draft)?;
        Ok(state)
    }

    fn _read(kind: &StateKind) -> anyhow::Result<State> {
        if let Some(content) = kind.read()? {
            let Some((exe_path, appname)) = content.split_once('\n') else {
                return Ok(Self::default());
            };
            let state = State {
                exe_path: Some(exe_path.to_string()),
                appname: Some(appname.to_string()),
            };
            Ok(state)
        } else {
            let state = Self::default();
            state._write(kind)?;
            Ok(state)
        }
    }

    pub fn write(&self) -> anyhow::Result<()> {
        self._write(&StateKind::Draft)?;
        Ok(())
    }

    fn _write(&self, kind: &StateKind) -> anyhow::Result<()> {
        kind.write(format!(
            "{}\n{}",
            self.exe_path.clone().unwrap_or_default(),
            self.appname.clone().unwrap_or_default(),
        ))?;
        Ok(())
    }

    pub fn use_older_state() -> anyhow::Result<()> {
        use StateKind::*;
        if Draft == Current {
            // draft:   old
            // current: old
            // old:     current
            Draft.replace_with(Old)?;
            Old.replace_with(Current)?;
            Current.replace_with(Draft)?;
        } else {
            // draft:   current
            // current: current
            // old:     draft
            Old.replace_with(Draft)?;
            Draft.replace_with(Current)?;
        }
        Ok(())
    }

    pub fn draft_to_current() -> anyhow::Result<()> {
        StateKind::Current.replace_with(StateKind::Draft)
    }
}
