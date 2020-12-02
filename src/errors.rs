use actix_web::{dev::HttpResponseBuilder, error, http::header, http::StatusCode, HttpResponse};
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum AppError {
    NotFound,
    InternalError,
    BadClientData,
    Timeout,
    Forbidden(String),
    SerdeError,
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        AppError::InternalError
    }
}

impl From<io::Error> for AppError {
    fn from(error: io::Error) -> Self {
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

impl error::ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        HttpResponseBuilder::new(self.status_code())
            .set_header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body("EEEE")
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::SerdeError | AppError::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::BadClientData => StatusCode::BAD_REQUEST,
            AppError::Timeout => StatusCode::GATEWAY_TIMEOUT,
            AppError::Forbidden(ref _e) => StatusCode::FORBIDDEN,
        }
    }
}
