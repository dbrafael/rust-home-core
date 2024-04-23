use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use http::Method;

use crate::server::{server::ServerRoute, ServerError, ServerRequest, ServerResult};

use super::{
    endpoint::Endpoint,
    parser::{PathArgumentMap, QueryPath, RoutePathToken, RoutePathTokenList},
    RequestHandler,
};

pub enum RequestType {
    REST(RequestHandler),
    Resource(String),
}

// ServerRouter is responsible for managing and resolving paths, both when registering and handling
#[derive(Default, Clone)]
pub struct Router {
    root: RouteNodeSafe,
}

impl Router {
    pub fn add(&mut self, route: ServerRoute) -> ServerResult<()> {
        RouteNode::register_path(self.root.clone(), route)?;
        Ok(())
    }

    pub fn get(&self, request: &ServerRequest) -> ServerResult<(RequestType, PathArgumentMap)> {
        let path = request.uri().path();
        let method = request.method().clone();
        let tokens: QueryPath = path.try_into()?;
        if tokens.resource.is_none() && method != Method::GET {
            return Err(ServerError::err("Invalid method"));
        }
        let (l_node, args) = tokens.resolve(self.root.clone())?;
        let node = l_node.lock().unwrap();
        println!("tokens: {:?}", tokens);
        println!("node: {:?}", node);
        match tokens.resource {
            None => Ok((RequestType::REST(node.endpoint.get(method)?), args)),
            Some(res) => match node.get_resource(&res) {
                Some(path) => Ok((RequestType::Resource(path.to_string()), args)),
                None => Err(ServerError::err("Resource not found")),
            },
        }
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
    resources: HashMap<String, String>, // name to fs path
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
    pub fn register_path(node: RouteNodeSafe, route: ServerRoute) -> ServerResult<()> {
        let mut path: RoutePathTokenList = match route {
            ServerRoute::REST(path, _, _) => path.try_into()?,
            ServerRoute::Resource(path, _) => path.try_into()?,
        };

        println!("Registering path: {:?}", path);

        let mut node = node;
        if path.tokens.len() > 0 {
            loop {
                let token = path.tokens.pop_front();
                node = RouteNode::add_or_get_single_node(node, token.unwrap())?;
                if path.tokens.is_empty() {
                    break;
                }
            }
        }
        let mut lock = node.lock().unwrap();
        match route {
            ServerRoute::REST(_, method, handler) => lock.register_method(method, handler),
            ServerRoute::Resource(name, path) => {
                lock.resources.insert(name.to_string(), path.to_string());
                Ok(())
            }
        }
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

    pub fn register_resource(&mut self, name: &str, path: &str) {
        self.resources.insert(name.to_string(), path.to_string());
    }

    pub fn get_resource(&self, name: &str) -> Option<&str> {
        self.resources.get(name).map(|s| s.as_str())
    }
}
