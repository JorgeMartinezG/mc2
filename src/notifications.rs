use serde::de;
use std::fmt::{self, Display};

#[derive(Debug)]
pub enum Notifications {
    SerdeError(String),
}

impl de::Error for Notifications {
    fn custom<T: Display>(msg: T) -> Self {
        Notifications::SerdeError(msg.to_string())
    }
}

impl Display for Notifications {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Notifications::SerdeError(msg) => formatter.write_str(msg),
        }
    }
}

impl std::error::Error for Notifications {}
