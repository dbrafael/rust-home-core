use tokio::net::TcpStream;

use super::{route::RequestHandler, RouteManager, ServerConfig, ServerResult};

pub trait HTTPServer {
    fn new(config: ServerConfig) -> Self;
    fn register(
        &mut self,
        path: &str,
        method: http::Method,
        handler: RequestHandler,
    ) -> ServerResult<()>;
    async fn start(&self) -> ServerResult<()>;
    async fn close(&self) -> ServerResult<()>;
    async fn handle(&self, stream: TcpStream) -> ServerResult<()>;
}

pub struct AsyncServer {
    config: ServerConfig,
    routes: RouteManager,
}

impl HTTPServer for AsyncServer {
    fn new(config: ServerConfig) -> Self {
        Self {
            config,
            routes: RouteManager::default(),
        }
    }

    fn register(
        &mut self,
        path: &str,
        method: http::Method,
        handler: RequestHandler,
    ) -> ServerResult<()> {
        self.routes.register(path, method, handler)
    }

    async fn start(&self) -> ServerResult<()> {
        unimplemented!()
    }

    async fn close(&self) -> ServerResult<()> {
        unimplemented!()
    }

    async fn handle(&self, stream: TcpStream) -> ServerResult<()> {
        unimplemented!()
    }
}
