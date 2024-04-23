mod inbound;
mod outbound;
mod router;
mod server;

use std::{
    collections::HashSet,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

use http::StatusCode;

pub use inbound::request::ServerRequest;
pub use outbound::response::{IntoResponse, ServerResponse};
pub use router::PathArgumentMap;
pub use server::HTTPServer;

use crate::common::log::{log_message, LogLevel};

use self::inbound::request::RequestArgs;

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

#[allow(dead_code)]
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Authentication {
    username: String,
    password: String,
}

impl Authentication {
    fn new(username: &str, password: &str) -> Self {
        Self {
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    pub fn from_request(request: &ServerRequest) -> ServerResult<Self> {
        let username = request.get_arg("username")?;
        let password = request.get_arg("password")?;
        Ok(Self::new(username, password))
    }
}

#[derive(Clone)]
pub struct ServerConfig {
    pub server_address: SocketAddr,
    pub allowed_addresses: HashSet<IpAddr>,
    pub allowed_users: HashSet<Authentication>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
            allowed_addresses: HashSet::new(),
            allowed_users: HashSet::new(),
        }
    }
}

impl ServerConfig {
    pub fn allow_address(&mut self, address: IpAddr) {
        self.allowed_addresses.insert(address);
    }
    pub fn allow_user(&mut self, user: &str, password: &str) {
        self.allowed_users
            .insert(Authentication::new(user, password));
    }

    pub fn allowed(&self, address: &IpAddr) -> bool {
        self.allowed_addresses.contains(address)
    }
    pub fn authenticate(&self, user: Authentication) -> bool {
        self.allowed_users.contains(&user)
    }
}
