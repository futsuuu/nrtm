use std::{
    env,
    process::{exit, Command},
};

use anyhow::Context as _;

use nrtm::shim::State;

fn main() -> anyhow::Result<()> {
    let state = State::read()?;
    State::draft_to_current()?;

    let exe_path = state.exe_path.context("Neovim is not installed.")?;
    let mut command = Command::new(exe_path);

    let appname = match env::var("NVIM_APPNAME") {
        Ok(appname) => Some(appname),
        Err(_) => state.appname,
    };
    if let Some(appname) = appname {
        command.env("NVIM_APPNAME", appname);
    }

    let exit_code = command.args(env::args_os().skip(1)).status()?.code();

    if let Some(code) = exit_code {
        exit(code);
    }

    Ok(())
}
