use crate::server::{request::ServerRequest, ServerError};
use std::collections::VecDeque;

// Route path is the path when registering a new endpoint, both static and variable tokens are
// stored by their name
#[derive(Debug, Clone)]
pub enum RoutePathToken {
    Static(&'static str),
    Variable(&'static str),
}

#[derive(Debug)]
pub struct RoutePath {
    tokens: VecDeque<RoutePathToken>,
}

impl Iterator for RoutePath {
    type Item = RoutePathToken;
    fn next(&mut self) -> Option<RoutePathToken> {
        self.tokens.pop_front()
    }
}

impl TryFrom<&'static str> for RoutePath {
    type Error = ServerError;
    fn try_from(value: &'static str) -> Result<Self, Self::Error> {
        let mut tokens = VecDeque::new();

        let str = value.trim_matches('/');
        let mut split = str.split("/");
        loop {
            let token = match split.next() {
                Some(t) => t,
                None => break,
            };
            if token.starts_with('[') && token.ends_with(']') {
                let token = token.trim_matches(|c| c == '[' || c == ']');
                if !token.chars().all(char::is_alphanumeric) {
                    return Err(ServerError::err("Invalid token").log());
                }
                tokens.push_back(RoutePathToken::Variable(token));
            } else if token.chars().all(char::is_alphanumeric) {
                tokens.push_back(RoutePathToken::Static(token));
            } else {
                return Err(ServerError::err("Error reading token").log());
            }
        }
        Ok(RoutePath { tokens })
    }
}

// Query path is the path when querying the system, variable token names' are unknown so the value
// of the token is actually the value of the variable
#[derive(Debug)]
pub struct QueryPath {
    pub tokens: VecDeque<String>,
    pub resource: Option<String>,
}

impl TryFrom<ServerRequest> for QueryPath {
    type Error = ServerError;
    fn try_from(value: ServerRequest) -> Result<Self, Self::Error> {
        let mut tokens = VecDeque::new();
        let mut resource = None;
        let path = value.path();
        let path_tokens = path.trim_matches('/').split('/');
        let mut size = path_tokens.clone().count();

        for token in path_tokens {
            if token.contains('.') {
                if !token.chars().all(|c| c.is_alphanumeric() || c == '.') {
                    return Err(ServerError::err("Invalid token").log());
                }
                if size == 1 {
                    resource = Some(token.to_string());
                } else {
                    return Err(ServerError::err("Invalid query").log());
                }
            } else {
                if !token.chars().all(char::is_alphanumeric) {
                    return Err(
                        ServerError::err(format!("Invalid token: {}", token).as_str()).log(),
                    );
                }
                tokens.push_back(token.to_string());
            }
            size -= 1;
        }
        Ok(QueryPath { tokens, resource })
    }
}

impl Iterator for QueryPath {
    type Item = String;
    fn next(&mut self) -> Option<String> {
        self.tokens.pop_front()
    }
}
