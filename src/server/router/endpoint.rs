use http::{Method, StatusCode};

use crate::server::{ServerError, ServerResult};

use super::RequestHandler;

// Endpoint holds callbacks for REST methods
#[derive(Debug, Clone)]
pub struct Endpoint {
    methods: [Option<RequestHandler>; 4], // Only GET, POST, UPDATE, DELETE for now
}

impl Default for Endpoint {
    fn default() -> Self {
        Self {
            methods: [None, None, None, None],
        }
    }
}

impl Endpoint {
    pub fn register(&mut self, method: Method, cb: RequestHandler) -> ServerResult<()> {
        let idx = method_to_num(method)?;
        if let Some(None) = self.methods.get(idx) {
            self.methods[idx] = Some(cb);
            return Ok(());
        }
        Err(ServerError::new(
            StatusCode::CONFLICT,
            "Method already exists",
        ))
    }

    pub fn get(&self, method: Method) -> ServerResult<RequestHandler> {
        match self.methods[method_to_num(method)?] {
            Some(callback) => Ok(callback),
            _ => Err(ServerError::new(StatusCode::NOT_FOUND, "Method not found")),
        }
    }
}

fn method_to_num(method: Method) -> ServerResult<usize> {
    Ok(match method {
        Method::GET => 0,
        Method::POST => 1,
        Method::PUT => 2,
        Method::DELETE => 3,
        _ => {
            return Err(ServerError::new(StatusCode::NOT_IMPLEMENTED, "Method not supported").log())
        }
    })
}
