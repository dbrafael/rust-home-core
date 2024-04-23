use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use http::Method;

use crate::server::{ServerError, ServerRequest, ServerResult};

use super::{
    endpoint::Endpoint,
    parser::{PathArgumentMap, QueryPathTokenList, RoutePathToken, RoutePathTokenList},
    RequestHandler,
};

// ServerRouter is responsible for managing and resolving paths, both when registering and handling
#[derive(Default, Clone)]
pub struct Router {
    root: RouteNodeSafe,
}

impl Router {
    pub fn add(&mut self, path: &str, method: Method, handler: RequestHandler) -> ServerResult<()> {
        RouteNode::register_path(self.root.clone(), path.try_into()?, method, handler)?;
        Ok(())
    }

    pub fn get(&self, request: &ServerRequest) -> ServerResult<(RequestHandler, PathArgumentMap)> {
        let path = request.uri().path();
        let method = request.method().clone();
        let tokens: QueryPathTokenList = path.try_into()?;
        let (l_node, args) = tokens.resolve(self.root.clone())?;
        let node = l_node.lock().unwrap();
        Ok((node.endpoint.get(method)?, args))
    }
}

// Routes can have a single variable children (/api/[userId]/...) or multiple static children
#[derive(Debug, Clone)]
enum RouteChildren {
    Static(HashMap<String, RouteNodeSafe>),
    Variable(String, RouteNodeSafe),
}

// A route node is a single token in the path
#[derive(Default, Debug, Clone)]
pub struct RouteNode {
    endpoint: Endpoint,
    children: Option<RouteChildren>,
}
pub type RouteNodeSafe = Arc<Mutex<RouteNode>>;

impl RouteNode {
    // These methods should be used when translating a user request to a path, var name is not
    // known by the user so we must infer the node type and return it accordingly
    pub fn get_var(node: RouteNodeSafe) -> Option<(String, RouteNodeSafe)> {
        let node = node.lock().unwrap();
        match &node.children {
            Some(RouteChildren::Variable(name, ptr)) => Some((name.to_string(), ptr.clone())),
            _ => None,
        }
    }

    // Likewise
    pub fn get_static(node: RouteNodeSafe, token: &str) -> Option<RouteNodeSafe> {
        let node = node.lock().unwrap();
        match &node.children {
            Some(RouteChildren::Static(map)) => map.get(token).cloned(),
            _ => None,
        }
    }

    // Registers a given path and creates missing sub-paths if possible/needed
    pub fn register_path(
        node: RouteNodeSafe,
        mut path: RoutePathTokenList,
        method: Method,
        callback: RequestHandler,
    ) -> ServerResult<()> {
        let mut node = node;
        loop {
            let token = path.0.pop_front();
            node = RouteNode::add_or_get_single_node(node, token.unwrap())?;
            if path.0.is_empty() {
                break;
            }
        }
        let mut lock = node.lock().unwrap();
        lock.register_method(method, callback)
    }

    pub fn register_method(&mut self, method: Method, handler: RequestHandler) -> ServerResult<()> {
        self.endpoint.register(method, handler)
    }

    fn add_or_get_single_node(
        p_node: RouteNodeSafe,
        token: RoutePathToken,
    ) -> ServerResult<RouteNodeSafe> {
        let mut node = p_node.lock().unwrap();
        match token {
            RoutePathToken::Static(name) => match &node.children {
                Some(RouteChildren::Variable(..)) => Err(ServerError::err("Route Conflict")),
                _ => {
                    let mut map = match node
                        .children
                        .get_or_insert(RouteChildren::Static(HashMap::new()))
                    {
                        RouteChildren::Static(map) => map.clone(),
                        _ => unreachable!(),
                    };
                    let ptr = Arc::new(Mutex::new(RouteNode::default()));
                    map.insert(name, ptr.clone());
                    node.children = Some(RouteChildren::Static(map));
                    Ok(ptr)
                }
            },
            RoutePathToken::Variable(name) => {
                if node.children.is_none() {
                    let ptr = Arc::new(Mutex::new(RouteNode::default()));
                    node.children = Some(RouteChildren::Variable(name, ptr.clone()));
                    Ok(ptr)
                } else {
                    Err(ServerError::err("Route Conflict"))
                }
            }
        }
    }
}
