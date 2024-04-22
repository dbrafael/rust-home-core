use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use http::Method;

use crate::server::{ServerError, ServerResult};

use super::{
    parser::{PathArgumentMap, QueryPathTokens, RoutePathToken, RoutePathTokens},
    route::ServerRoute,
    RequestHandler,
};

pub struct RouteManager {
    root: Arc<Mutex<RouteNode>>,
}

impl Default for RouteManager {
    fn default() -> Self {
        Self {
            root: Arc::new(Mutex::new(RouteNode::default())),
        }
    }
}

impl RouteManager {
    pub fn register(
        &mut self,
        path: &str,
        method: Method,
        handler: RequestHandler,
    ) -> ServerResult<()> {
        let tokens: RoutePathTokens = path.try_into()?;
        RouteNode::get_add_missing(self.root.clone(), tokens, method, handler)?;
        Ok(())
    }

    pub fn get(
        &self,
        path: &str,
        method: Method,
    ) -> ServerResult<(RequestHandler, PathArgumentMap)> {
        let tokens: QueryPathTokens = path.try_into()?;
        let (l_node, args) = tokens.resolve(self.root.clone())?;
        let node = l_node.lock().unwrap();
        Ok((node.handler.get(method)?, args))
    }
}

#[derive(Debug, Clone)]
enum RouteChildren {
    Static(HashMap<String, Arc<Mutex<RouteNode>>>),
    Variable(String, Arc<Mutex<RouteNode>>),
}

#[derive(Debug, Clone)]
pub struct RouteNode {
    handler: ServerRoute,
    children: Option<RouteChildren>,
}

impl Default for RouteNode {
    fn default() -> Self {
        Self {
            handler: ServerRoute::default(),
            children: None,
        }
    }
}

impl RouteNode {
    pub fn get_var(node: Arc<Mutex<RouteNode>>) -> Option<(String, Arc<Mutex<RouteNode>>)> {
        let node = node.lock().unwrap();
        match &node.children {
            Some(RouteChildren::Variable(name, ptr)) => Some((name.to_string(), ptr.clone())),
            _ => None,
        }
    }

    pub fn get_static(node: Arc<Mutex<RouteNode>>, token: &str) -> Option<Arc<Mutex<RouteNode>>> {
        let node = node.lock().unwrap();
        match &node.children {
            Some(RouteChildren::Static(map)) => map.get(token).cloned(),
            _ => None,
        }
    }

    pub fn get_add_missing(
        node: Arc<Mutex<RouteNode>>,
        mut path: RoutePathTokens,
        method: Method,
        callback: RequestHandler,
    ) -> ServerResult<()> {
        let mut node = node;
        loop {
            let token = path.0.pop_front();
            node = RouteNode::get_add_missing_single_node(node, token.unwrap())?;
            if path.0.is_empty() {
                break;
            }
        }
        let mut lock = node.lock().unwrap();
        lock.add_method(method, callback)
    }

    pub fn add_method(&mut self, method: Method, handler: RequestHandler) -> ServerResult<()> {
        self.handler.register(method, handler)
    }

    fn get_add_missing_single_node(
        p_node: Arc<Mutex<RouteNode>>,
        token: RoutePathToken,
    ) -> ServerResult<Arc<Mutex<RouteNode>>> {
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
