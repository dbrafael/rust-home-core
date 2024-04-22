use std::collections::HashMap;

use http::{Method, StatusCode};

use crate::server::{ServerError, ServerResult};

use super::RequestHandler;

#[derive(Debug, Clone)]
pub struct ServerRoute {
    handlers: HashMap<Method, RequestHandler>,
}

impl Default for ServerRoute {
    fn default() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }
}

impl ServerRoute {
    pub fn register(&mut self, method: Method, cb: RequestHandler) -> ServerResult<()> {
        if let None = self.handlers.get(&method) {
            self.handlers.insert(method, cb);
            return Ok(());
        }
        Err(ServerError::new(
            StatusCode::CONFLICT,
            "Method already exists",
        ))
    }

    pub fn get(&self, method: Method) -> ServerResult<RequestHandler> {
        if let Some(cb) = self.handlers.get(&method) {
            return Ok(*cb);
        }
        Err(ServerError::new(StatusCode::NOT_FOUND, "Method not found"))
    }
}
