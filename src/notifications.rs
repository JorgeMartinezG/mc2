use serde::de;
use std::fmt::{self, Display};
use std::io;

#[derive(Debug)]
pub enum Notifications {
    SerdeError(String),
    IOError(String),
}

impl From<io::Error> for Notifications {
    fn from(error: io::Error) -> Self {
        Notifications::IOError(error.to_string())
    }
}

impl de::Error for Notifications {
    fn custom<T: Display>(msg: T) -> Self {
        Notifications::SerdeError(msg.to_string())
    }
}

impl Display for Notifications {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Notifications::SerdeError(msg) => {
                formatter.write_str(format!("SERDE::{}", msg).as_str())
            }
            Notifications::IOError(msg) => formatter.write_str(format!("IO::{}", msg).as_str()),
        }
    }
}

impl std::error::Error for Notifications {}
