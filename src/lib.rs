pub mod libmpv_handler;
pub mod logger;
pub mod mc_os_interface;
pub mod tui;

use crate::libmpv_handler::{LibMpvEventMessage, LibMpvMessage};

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

    let last_arg = args
        .pop()
        .or_else(|| std::fs::read_to_string("last.txt").ok())
        .ok_or(UAPlayerError::InvalidOptionsStructure)?;
    if last_arg != "--help" {
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
    } else {
        args.push(last_arg);
    }

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
