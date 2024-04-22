mod request;
mod response;
mod route;
mod server;

use std::{
    collections::HashSet,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

use http::StatusCode;

pub use request::ServerRequest;
pub use response::{IntoResponse, ServerResponse};
pub use route::manager::RouteManager;
pub use route::parser::PathArgumentMap;

#[allow(dead_code)]
#[derive(Debug)]
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
}

pub type ServerResult<T> = Result<T, ServerError>;
#[allow(dead_code)]
pub struct Authentication<'a> {
    username: &'a str,
    password: &'a str,
}

pub struct ServerConfig {
    pub server_address: SocketAddr,
    pub allowed_addresses: HashSet<SocketAddr>,
    pub allowed_users: HashSet<Authentication<'static>>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 0)), 8080),
            allowed_addresses: HashSet::new(),
            allowed_users: HashSet::new(),
        }
    }
}
