use std::{collections::HashSet, net::IpAddr};

use super::{request::ServerRequest, ServerResult};

#[allow(dead_code)]
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Authentication {
    username: String,
    password: String,
}

impl Authentication {
    pub fn new(username: &str, password: &str) -> Self {
        Self {
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    pub fn from_request(request: &ServerRequest) -> ServerResult<Self> {
        let username = request.query_argument("username")?;
        let password = request.query_argument("password")?;
        Ok(Self::new(username, password))
    }
}

#[derive(Clone)]
pub struct AuthManager {
    allowed_addresses: HashSet<IpAddr>,
    allowed_users: HashSet<Authentication>,
}

impl AuthManager {
    pub fn allows(&self, ip: IpAddr) -> bool {
        self.allowed_addresses.contains(&ip)
    }

    pub fn authenticate(&self, user: &Authentication) -> bool {
        self.allowed_users.contains(user)
    }
}

pub struct AuthBuilder {
    auth: AuthManager,
}

impl AuthBuilder {
    pub fn new() -> Self {
        Self {
            auth: AuthManager {
                allowed_addresses: HashSet::new(),
                allowed_users: HashSet::new(),
            },
        }
    }

    pub fn allow_address(&mut self, address: IpAddr) -> &mut Self {
        self.auth.allowed_addresses.insert(address);
        self
    }

    pub fn allow_user(&mut self, user: Authentication) -> &mut Self {
        self.auth.allowed_users.insert(user);
        self
    }

    pub fn build(&self) -> AuthManager {
        self.auth.clone()
    }
}
