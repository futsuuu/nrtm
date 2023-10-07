use std::{
    env,
    process::{exit, Command},
};

use nrtm::shim;

fn main() -> anyhow::Result<()> {
    let state = shim::State::read()?;
    let mut command = Command::new(state.exe_path);
    if !state.appname.is_empty() && env::var("NVIM_APPNAME").is_err() {
        command.env("NVIM_APPNAME", state.appname);
    }
    let exit_code = command.args(env::args_os().skip(1)).status()?.code();

    if let Some(code) = exit_code {
        exit(code);
    }

    Ok(())
}
