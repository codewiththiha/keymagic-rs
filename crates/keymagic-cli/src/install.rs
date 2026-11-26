//! `keymagic install --user`: writes the `IBus` component file for this user.

use keymagic_host::Paths;
use keymagic_host::config::Config;
use keymagic_host::integration::ibus;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

/// Writes the component file under the user's data folder.
pub fn user() -> Result<ExitCode, String> {
    if !cfg!(target_os = "linux") {
        return Err("install --user writes IBus files and runs on Linux only".to_owned());
    }
    let paths = Paths::for_user().ok_or("no user folders for this account")?;
    let config =
        Config::load_or_default(&paths.config_file()).map_err(|error| error.to_string())?;
    let engines = ibus::engines(&config, &paths);
    let file = ibus::user_component_path(&data_home()?);
    if let Some(folder) = file.parent() {
        fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    }
    let exec = engine_exec()?;
    fs::write(&file, ibus::component_xml(&exec, &engines)).map_err(|error| error.to_string())?;
    println!(
        "Wrote {} with {} engine(s). Restart IBus so it reads the file.",
        file.display(),
        engines.len()
    );
    Ok(ExitCode::SUCCESS)
}

/// Engine path and argument; `IBus` has no PATH lookup and splits on spaces.
fn engine_exec() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let engine = exe
        .parent()
        .ok_or("cannot find the keymagic program folder")?
        .join("keymagic-ibus");
    if !engine.is_file() {
        return Err(format!(
            "{} is missing; install keymagic-ibus beside keymagic",
            engine.display()
        ));
    }
    let path = engine.to_string_lossy();
    if path.contains(char::is_whitespace) {
        return Err(format!("{path} has spaces; IBus cannot start it there"));
    }
    Ok(format!("{path} --ibus"))
}

/// `$XDG_DATA_HOME`, or `~/.local/share` when it is unset.
fn data_home() -> Result<PathBuf, String> {
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME").filter(|dir| !dir.is_empty()) {
        return Ok(PathBuf::from(dir));
    }
    let home = std::env::var_os("HOME").ok_or("HOME is not set")?;
    Ok(PathBuf::from(home).join(".local/share"))
}
