pub mod libmpv_handler;
pub mod logger;
pub mod mc_os_interface;
pub mod tui;

use crate::libmpv_handler::{LibMpvEventMessage, LibMpvMessage};
#[cfg(not(target_os = "linux"))]
use std::io::Write;

#[derive(Debug)]
pub enum UAPlayerError {
    InvalidOption(String),
    InvalidOptionsStructure,
    InvalidFile,
    SouvlakiError(souvlaki::Error),
    SystemTimeError(std::time::SystemTimeError),
    IOError(std::io::Error),
    LibMpvMessageSendError(crossbeam::channel::SendError<LibMpvMessage>),
    LibMpvEventMessageSendError(crossbeam::channel::SendError<LibMpvEventMessage>),
    LibMpvError(libmpv2::Error),
}

impl From<souvlaki::Error> for UAPlayerError {
    fn from(err: souvlaki::Error) -> Self {
        UAPlayerError::SouvlakiError(err)
    }
}

impl From<std::time::SystemTimeError> for UAPlayerError {
    fn from(err: std::time::SystemTimeError) -> Self {
        UAPlayerError::SystemTimeError(err)
    }
}

impl From<std::io::Error> for UAPlayerError {
    fn from(err: std::io::Error) -> Self {
        UAPlayerError::IOError(err)
    }
}

impl From<crossbeam::channel::SendError<LibMpvMessage>> for UAPlayerError {
    fn from(err: crossbeam::channel::SendError<LibMpvMessage>) -> Self {
        UAPlayerError::LibMpvMessageSendError(err)
    }
}

impl From<crossbeam::channel::SendError<LibMpvEventMessage>> for UAPlayerError {
    fn from(err: crossbeam::channel::SendError<LibMpvEventMessage>) -> Self {
        UAPlayerError::LibMpvEventMessageSendError(err)
    }
}

impl From<libmpv2::Error> for UAPlayerError {
    fn from(err: libmpv2::Error) -> Self {
        UAPlayerError::LibMpvError(err)
    }
}

#[derive(PartialEq)]
pub enum ProgramOption {
    PATH(String),
    PrintHelp,
    Volume(i64),
    Verbose,
}

pub fn process_args() -> Result<Vec<ProgramOption>, UAPlayerError> {
    let mut options = vec![];
    let mut args: Vec<String> = std::env::args().skip(1).collect();

    let mut last_arg = args
        .pop()
        .or_else(|| load_path_from_config())
        .ok_or(UAPlayerError::InvalidOptionsStructure)?;

    if last_arg == "--help" {
        options.push(ProgramOption::PrintHelp);
        return Ok(options);
    }

    if last_arg.starts_with("--") {
        args.push(last_arg);
        last_arg = load_path_from_config().ok_or(UAPlayerError::InvalidOptionsStructure)?;
    }

    let file_path = last_arg;
    let abs_file_path = std::path::absolute(&file_path)?;
    if !abs_file_path.try_exists()? {
        return Err(UAPlayerError::InvalidFile);
    }
    if !is_audiofile(&abs_file_path) {
        return Err(UAPlayerError::InvalidFile);
    }

    options.push(ProgramOption::PATH(
        abs_file_path.to_string_lossy().to_string(),
    ));

    for arg in args {
        let arg = match arg.as_str() {
            "--help" => Ok(ProgramOption::PrintHelp),
            "--verbose" => Ok(ProgramOption::Verbose),
            s if s.starts_with("--volume=") => {
                if let Some(Ok(vol)) = s.split_once('=').map(|(_, s)| s.parse::<i8>()) {
                    if (0..=100).contains(&vol) {
                        Ok(ProgramOption::Volume(vol.into()))
                    } else {
                        Err(UAPlayerError::InvalidOption(arg))
                    }
                } else {
                    Err(UAPlayerError::InvalidOption(arg))
                }
            }
            _ => Err(UAPlayerError::InvalidOption(arg)),
        };
        options.push(arg?);
    }

    Ok(options)
}

#[cfg(target_os = "linux")]
fn load_path_from_config() -> Option<String> {
    let config_file_path = std::env::var("XDG_CONFIG_HOME")
        .or(std::env::var("HOME").map(|s| format!("{s}/.config")))
        .map(|path| format!("{path}/{}/config", env!("CARGO_PKG_NAME")));
    if let Ok(path) = config_file_path {
        if std::path::PathBuf::from(&path).is_file() {
            return std::fs::read_to_string(path).ok();
        }
    }

    None
}

#[cfg(target_os = "linux")]
pub fn save_path_to_config(path: &str) {
    let config_dir_path = std::env::var("XDG_CONFIG_HOME")
        .or(std::env::var("HOME").map(|s| format!("{s}/.config")))
        .map(|path| format!("{path}/{}", env!("CARGO_PKG_NAME")));
    if let Ok(dir_path) = config_dir_path {
        if !std::path::PathBuf::from(&dir_path).is_dir() {
            std::fs::create_dir(dir_path.clone()).unwrap();
        }
        let config_file_path = format!("{dir_path}/config");
        std::fs::write(config_file_path, path).unwrap();
    }
}

#[cfg(target_os = "windows")]
fn load_path_from_config() -> Option<String> {
    let config_file_path =
        std::env::var("APPDATA").map(|path| format!("{path}/{}/config", env!("CARGO_PKG_NAME")));
    if let Ok(path) = config_file_path {
        if std::path::PathBuf::from(&path).is_file() {
            return std::fs::read_to_string(path).ok();
        }
    }

    None
}

#[cfg(target_os = "windows")]
pub fn save_path_to_config(path: &str) {
    let config_dir_path =
        std::env::var("APPDATA").map(|path| format!("{path}/{}", env!("CARGO_PKG_NAME")));
    if let Ok(dir_path) = config_dir_path {
        if !std::path::PathBuf::from(&dir_path).is_dir() {
            std::fs::create_dir(dir_path.clone()).unwrap();
        }
        let config_file_path = format!("{dir_path}/config");
        std::fs::write(config_file_path, path).unwrap();
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn load_path_from_config() -> Option<String> {
    std::fs::read_to_string("last.txt").ok()
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn save_path_to_config(path: &str) {
    let mut file = std::fs::File::create(format!("last.txt")).unwrap();
    file.write_all(path.as_bytes()).unwrap();
    log::debug!("File path: {path}");
}

pub fn print_help() {
    println!("Usage: {} [OPTIONS] [PATH]", env!("CARGO_PKG_NAME"));
    println!("       {} --help", env!("CARGO_PKG_NAME"));
    println!("Options:");
    println!("\t --volume=<value>\t(0..100)");
    println!("\t --verbose");
    println!("\t --help");
}

fn is_audiofile(path: &std::path::PathBuf) -> bool {
    if let Some(ext) = path.extension() {
        if ext == "m4b" {
            return true;
        } else if ext == "mp3" {
            return true;
        }
    }

    false
}
