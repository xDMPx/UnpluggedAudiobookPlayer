pub mod libmpv_handler;
pub mod mc_os_interface;
pub mod tui;

#[derive(Debug)]
pub enum UAPlayerError {
    SouvlakiError(souvlaki::Error),
    SystemTimeError(std::time::SystemTimeError),
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
