use crate::server::{ServerError, ServerResult};
use std::collections::{HashMap, VecDeque};

use super::router::{RouteNode, RouteNodeSafe};

// Route path is the path when registering a new endpoint, both static and variable tokens are
// stored by their name
#[derive(Debug, Clone)]
pub enum RoutePathToken {
    Static(String),
    Variable(String),
}
#[derive(Debug)]
pub struct RoutePathTokenList {
    pub tokens: VecDeque<RoutePathToken>,
    pub resource: Option<String>,
}

impl TryFrom<&str> for RoutePathTokenList {
    type Error = ServerError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut tokens = VecDeque::new();
        let mut resource = None;

        let str = value.trim_matches('/');
        let mut split = str.split("/");
        loop {
            let tok = match split.next() {
                Some(t) => t,
                None => break,
            };
            if tok.contains('.') {
                if !tok.chars().all(|c| c.is_alphanumeric() || c == '.') {
                    return Err(ServerError::err("Invalid token").log());
                }
                if split.count() == 0 {
                    resource = Some(tok.to_string());
                    break;
                }
                return Err(ServerError::err("Invalid query").log());
            };
            if tok.starts_with('[') && tok.ends_with(']') {
                let token = tok.trim_matches(|c| c == '[' || c == ']');
                if !token.chars().all(char::is_alphanumeric) {
                    return Err(ServerError::err("Invalid token").log());
                }
                tokens.push_back(RoutePathToken::Variable(token.to_string()));
            } else if tok.chars().all(char::is_alphanumeric) {
                tokens.push_back(RoutePathToken::Static(tok.to_string()));
            } else {
                return Err(ServerError::err("Error reading token").log());
            }
        }
        Ok(RoutePathTokenList { tokens, resource })
    }
}

// Query path is the path when querying the system, variable token names' are unknown so the value
// of the token is actually the value of the variable
#[derive(Debug)]
pub struct QueryPath {
    pub tokens: Vec<String>,
    pub resource: Option<String>,
}

impl TryFrom<&str> for QueryPath {
    type Error = ServerError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut tokens = Vec::new();
        let mut resource = None;

        let str = value.trim_matches('/');
        let mut split = str.split("/");
        loop {
            let tok = match split.next() {
                Some(t) => t,
                None => break,
            };
            if tok.contains('.') {
                if !tok.chars().all(|c| c.is_alphanumeric() || c == '.') {
                    return Err(ServerError::err("Invalid token").log());
                }
                if split.count() == 0 {
                    resource = Some(tok.to_string());
                    break;
                }
                return Err(ServerError::err("Invalid query").log());
            };
            if !tok.chars().all(char::is_alphanumeric) {
                return Err(ServerError::err(format!("Invalid token: {}", tok).as_str()).log());
            }
            tokens.push(tok.to_string());
        }
        Ok(QueryPath { tokens, resource })
    }
}

pub type PathArgumentMap = HashMap<String, String>;

impl QueryPath {
    pub fn resolve(&self, root: RouteNodeSafe) -> ServerResult<(RouteNodeSafe, PathArgumentMap)> {
        let mut current_ptr = root;
        let mut args = HashMap::new();
        for token in self.tokens.iter() {
            if let Some((name, child)) = RouteNode::get_var(current_ptr.clone()) {
                args.insert(name, token.clone());
                current_ptr = child;
            } else if let Some(child) = RouteNode::get_static(current_ptr, token) {
                current_ptr = child;
            } else {
                return Err(ServerError::err("Invalid path").log());
            }
        }
        Ok((current_ptr, args))
    }
}
