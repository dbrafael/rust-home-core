use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, Mutex},
};

use http::Method;

use crate::server::{
    request::ServerRequest,
    response::{IntoResponse, ServerResponse},
    router::parser::RoutePath,
    ServerError, ServerResult,
};

use super::{
    parser::{QueryPath, RoutePathToken},
    RequestHandler,
};

// ServerRouter is responsible for managing and resolving paths, both when registering and handling
#[derive(Default, Debug)]
pub struct Router {
    routes: RouteTree,
}

// Route tree holds the data for the path tree
#[derive(Default, Clone, Debug)]
struct RouteTree {
    root: RouteNodePointer,
}

// A route node is a single node in the tree, it corresponds to a token in the path (ex:
// /api/test/ -> both "api" and "test" have a corresponding node
#[derive(Default, Clone, Debug)]
pub struct RouteNode {
    rest: [Option<RequestHandler>; 4],
    children: Option<RouteChildren>,
    resources: HashMap<ResourceName<'static>, ResourceLocation<'static>>,
}
pub type RouteNodePointer = Arc<Mutex<RouteNode>>;

// Routes can have a single variable children (/api/[userId]/...) or multiple static children
#[derive(Debug, Clone)]
enum RouteChildren {
    Static(HashMap<&'static str, RouteNodePointer>),
    Variable(&'static str, RouteNodePointer),
}

pub type ResourceName<'a> = &'a str;
pub type ResourceLocation<'a> = &'a str;

enum NodeEndpoint {
    REST(Method, RequestHandler),
    Resource(ResourceName<'static>, ResourceLocation<'static>),
}

pub type PathToken = String;
pub type TokenValue = String;

pub type PathArguments = HashMap<PathToken, TokenValue>;

impl Router {
    pub fn resolve(&self, request: ServerRequest) -> ServerResult<ServerResponse> {
        let path: QueryPath = request.clone().try_into()?;
        let (node_p, args) = self.routes.get(&path)?;
        let node = node_p.lock().unwrap();
        if let Some(resource) = path.resource {
            let real_path = node
                .get_resource(&resource)
                .ok_or(ServerError::err("Resource not found"))?;
            Ok(ServerResponse::file(real_path)?)
        } else {
            let handler = node
                .get_rest(request.method().clone())
                .ok_or(ServerError::err("Method not found"))?;
            handler(request, args)
        }
    }
}

impl RouteNode {
    fn get_resource(&self, name: ResourceName) -> Option<&ResourceLocation> {
        self.resources.get(name)
    }

    fn get_rest(&self, method: Method) -> Option<&RequestHandler> {
        self.rest[method_as_usize(method)].as_ref()
    }

    fn get_child_var(&self) -> Option<(String, RouteNodePointer)> {
        match &self.children {
            Some(RouteChildren::Variable(name, ptr)) => Some((name.to_string(), ptr.clone())),
            _ => None,
        }
    }

    fn get_child_static(&self, token: &str) -> Option<RouteNodePointer> {
        match &self.children {
            Some(RouteChildren::Static(map)) => map.get(token).cloned(),
            _ => None,
        }
    }

    fn register_endpoint(ptr: RouteNodePointer, req: NodeEndpoint) -> ServerResult<()> {
        let mut node = ptr.lock().unwrap();
        match req {
            NodeEndpoint::REST(method, callback) => {
                Ok(node.rest[method_as_usize(method)] = Some(callback))
            }
            NodeEndpoint::Resource(name, loc) => {
                if let Some(_) = node.resources.get(&name) {
                    return Err(ServerError::err("Resource Conflict"));
                }
                node.resources.insert(name, loc);
                Ok(())
            }
        }
    }

    fn register_static_child(
        ptr: RouteNodePointer,
        token: &'static str,
    ) -> ServerResult<RouteNodePointer> {
        let mut node = ptr.lock().unwrap();
        if let Some(_) = node.get_child_var() {
            return Err(ServerError::err("Route Conflict"));
        }
        if let None = node.children {
            node.children = Some(RouteChildren::Static(HashMap::new()));
        }
        match node.children.as_mut() {
            Some(RouteChildren::Static(children)) => {
                if let Some(_) = children.get(token) {
                    return Err(ServerError::err("Route Conflict"));
                }
                let ptr = Arc::new(Mutex::new(RouteNode::default()));
                let _ = children.insert(token, ptr.clone());
                Ok(ptr)
            }
            _ => unreachable!(),
        }
    }

    fn register_variable_child(
        ptr: RouteNodePointer,
        token: &'static str,
    ) -> ServerResult<RouteNodePointer> {
        let mut node = ptr.lock().unwrap();
        if let Some(_) = node.children {
            return Err(ServerError::err("Route Conflict"));
        }
        let ptr = Arc::new(Mutex::new(RouteNode::default()));
        node.children = Some(RouteChildren::Variable(token, ptr.clone()));
        Ok(ptr)
    }

    fn ensure_child(
        ptr: RouteNodePointer,
        token: RoutePathToken,
    ) -> ServerResult<RouteNodePointer> {
        match token {
            RoutePathToken::Static(name) => Self::register_static_child(ptr, name),
            RoutePathToken::Variable(name) => Self::register_variable_child(ptr, name),
        }
    }
}

impl RouteTree {
    fn ensure_path(&mut self, path: RoutePath) -> ServerResult<RouteNodePointer> {
        let mut ptr = self.root.clone();
        for token in path {
            ptr = RouteNode::ensure_child(ptr, token)?;
        }
        Ok(ptr)
    }

    fn register(&mut self, path: &'static str, request: NodeEndpoint) -> ServerResult<()> {
        println!("Registering path: {:?}", path);
        let ptr = match path {
            "/" => self.root.clone(),
            _ => self.ensure_path(path.try_into()?)?,
        };
        RouteNode::register_endpoint(ptr, request)
    }

    pub fn get(&self, path: &QueryPath) -> ServerResult<(RouteNodePointer, PathArguments)> {
        let mut next = self.root.clone();
        let mut ptr: RouteNodePointer;
        let mut args = HashMap::new();
        for token in &path.tokens {
            ptr = next;
            let node = ptr.lock().unwrap();
            if let Some((name, child)) = node.get_child_var() {
                args.insert(name, token.clone().to_string());
                next = child;
            } else if let Some(child) = node.get_child_static(&token) {
                next = child;
            } else {
                return Err(ServerError::err("Invalid path").log());
            }
        }
        Ok((next, args))
    }
}

#[derive(Default, Clone)]
pub struct RouterBuilder {
    tree: RouteTree,
}

#[allow(dead_code)]
impl RouterBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    fn rest(
        &mut self,
        path: &'static str,
        method: Method,
        handler: RequestHandler,
    ) -> ServerResult<&mut Self> {
        match self
            .tree
            .register(path, NodeEndpoint::REST(method, handler))
        {
            Ok(_) => Ok(self),
            Err(e) => Err(e),
        }
    }

    pub fn get(&mut self, path: &'static str, handler: RequestHandler) -> &mut Self {
        self.rest(path, Method::GET, handler).unwrap()
    }

    pub fn post(&mut self, path: &'static str, handler: RequestHandler) -> &mut Self {
        self.rest(path, Method::POST, handler).unwrap()
    }

    pub fn put(&mut self, path: &'static str, handler: RequestHandler) -> &mut Self {
        self.rest(path, Method::PUT, handler).unwrap()
    }

    pub fn delete(&mut self, path: &'static str, handler: RequestHandler) -> &mut Self {
        self.rest(path, Method::DELETE, handler).unwrap()
    }

    pub fn resource(
        &mut self,
        path: &'static str,
        name: ResourceName<'static>,
        loc: ResourceLocation<'static>,
    ) -> &mut Self {
        self.tree
            .register(path, NodeEndpoint::Resource(name, loc))
            .unwrap();
        self
    }

    pub fn build(&self) -> Router {
        Router {
            routes: self.tree.clone(),
        }
    }
}

fn method_as_usize(method: Method) -> usize {
    match method {
        Method::GET => 0,
        Method::POST => 1,
        Method::PUT => 2,
        Method::DELETE => 3,
        _ => unimplemented!("Only GET/POST/PUT/DELETE supported"),
    }
}
