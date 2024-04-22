use super::manager::RouteNode;
use crate::server::{ServerError, ServerResult};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone)]
pub enum RoutePathToken {
    Static(String),
    Variable(String),
}

#[derive(Debug)]
pub struct RoutePathTokens(pub VecDeque<RoutePathToken>);
#[derive(Debug)]
pub struct QueryPathTokens(Vec<String>);

impl TryFrom<&str> for RoutePathTokens {
    type Error = ServerError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut tokens = VecDeque::new();

        let str = value.trim_matches('/');
        let split = str.split("/");
        for tok in split {
            if tok.starts_with('[') && tok.ends_with(']') {
                let token = tok.trim_matches(|c| c == '[' || c == ']');
                if !token.chars().all(char::is_alphanumeric) {
                    return Err(ServerError::err("Invalid token"));
                }
                tokens.push_back(RoutePathToken::Variable(token.to_string()));
            } else if tok.chars().all(char::is_alphanumeric) {
                tokens.push_back(RoutePathToken::Static(tok.to_string()));
            } else {
                return Err(ServerError::err("Error reading token"));
            }
        }
        Ok(RoutePathTokens(tokens))
    }
}

impl TryFrom<&str> for QueryPathTokens {
    type Error = ServerError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut tokens = Vec::new();

        let str = value.trim_matches('/');
        let split = str.split("/");
        for tok in split {
            if !tok.chars().all(char::is_alphanumeric) {
                return Err(ServerError::err(format!("Invalid token: {}", tok).as_str()));
            }
            tokens.push(tok.to_string());
        }
        Ok(QueryPathTokens(tokens))
    }
}

pub type PathArgumentMap = HashMap<String, String>;

impl QueryPathTokens {
    pub fn resolve(
        self,
        root: Arc<Mutex<RouteNode>>,
    ) -> ServerResult<(Arc<Mutex<RouteNode>>, PathArgumentMap)> {
        let mut current_ptr = root;
        let mut args = HashMap::new();
        for token in self.0.iter() {
            if let Some((name, child)) = RouteNode::get_var(current_ptr.clone()) {
                args.insert(name, token.clone());
                current_ptr = child;
            } else if let Some(child) = RouteNode::get_static(current_ptr, token) {
                current_ptr = child;
            } else {
                return Err(ServerError::err("Invalid path"));
            }
        }
        Ok((current_ptr, args))
    }
}
