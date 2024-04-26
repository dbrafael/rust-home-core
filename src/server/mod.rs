pub mod auth;
pub mod connection;
pub mod request;
pub mod response;
pub mod router;
pub mod server;

use crate::common::log::{log_message, LogLevel};
use http::StatusCode;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ServerError {
    code: StatusCode,
    error: String,
}

impl ServerError {
    pub fn new(code: StatusCode, error: &str) -> Self {
        ServerError {
            code,
            error: error.to_string(),
        }
    }

    pub fn err(error: &str) -> Self {
        ServerError::new(StatusCode::INTERNAL_SERVER_ERROR, error)
    }

    pub fn log(self) -> Self {
        log_message(LogLevel::Error, &self.error);
        self
    }
}

pub type ServerResult<T> = Result<T, ServerError>;
