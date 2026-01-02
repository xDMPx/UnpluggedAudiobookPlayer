pub mod libmpv_handler;
pub mod mc_os_interface;
pub mod tui;

use crate::libmpv_handler::{LibMpvEventMessage, LibMpvMessage};

#[derive(Debug)]
pub enum UAPlayerError {
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
