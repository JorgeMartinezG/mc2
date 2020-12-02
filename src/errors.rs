use std::fmt;
use std::io;

use log::error;

#[derive(Debug)]
pub enum AppError {
    NotFound,
    InternalError,
    Forbidden,
    SerdeError(String),
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        AppError::SerdeError(error.to_string())
    }
}

impl From<io::Error> for AppError {
    fn from(error: io::Error) -> Self {
        error!("{:?}", error.to_string());
        match error.kind() {
            io::ErrorKind::NotFound => AppError::NotFound,
            _ => AppError::InternalError,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AAA")
    }
}
