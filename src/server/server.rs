use std::sync::Arc;

use http::{Method, StatusCode};
use tokio::{
    net::TcpListener,
    sync::{OwnedSemaphorePermit, Semaphore},
    task::JoinHandle,
};

use super::{
    connection::Connection,
    router::{RequestHandler, RequestType, Router},
    Authentication, IntoResponse, ServerConfig, ServerError, ServerResponse, ServerResult,
};

pub struct HTTPServer {
    config: ServerConfig,
    router: Router,
    worker_pool: Arc<Semaphore>,
}

pub enum ServerRoute {
    REST(&'static str, Method, RequestHandler),
    Resource(&'static str, &'static str),
}

impl HTTPServer {
    pub fn new(config: ServerConfig, routes: Vec<ServerRoute>) -> ServerResult<Self> {
        let mut router = Router::default();
        for route in routes {
            router.add(route)?;
        }
        Ok(Self {
            config,
            router,
            worker_pool: Arc::new(Semaphore::new(5)),
        })
    }

    pub async fn start(self) -> ServerResult<JoinHandle<()>> {
        let listener = match TcpListener::bind(self.config.server_address).await {
            Ok(l) => l,
            Err(e) => {
                return Err(ServerError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Error binding to address: {}", e),
                ));
            }
        };
        let handle = tokio::spawn(async move {
            println!("Server started at: {}", self.config.server_address);
            loop {
                if let Ok(con) = self.wait_request(&listener).await {
                    let permit = match self.worker_pool.clone().acquire_owned().await {
                        Ok(p) => p,
                        Err(e) => {
                            println!("Error acquiring semaphore: {}", e);
                            continue;
                        }
                    };
                    ConnectionWorker::spawn(self.config.clone(), self.router.clone(), con, permit);
                } else {
                    println!("Error accepting connection");
                }
            }
        });
        Ok(handle)
    }

    async fn wait_request(&self, listener: &TcpListener) -> ServerResult<Connection> {
        match listener.accept().await {
            Ok((stream, addr)) => Ok(Connection::from_connection(stream, addr)),
            Err(e) => {
                return Err(ServerError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Error accepting connection: {}", e),
                ));
            }
        }
    }
}

struct ConnectionWorker {
    config: ServerConfig,
    router: Router,
    connection: Connection,
}

impl ConnectionWorker {
    pub fn spawn(
        config: ServerConfig,
        router: Router,
        connection: Connection,
        permit: OwnedSemaphorePermit,
    ) -> JoinHandle<ServerResult<()>> {
        let mut worker = Self {
            config,
            router,
            connection,
        };
        tokio::spawn(async move { worker.start(permit).await })
    }

    async fn start(&mut self, permit: OwnedSemaphorePermit) -> ServerResult<()> {
        println!("Connection from: {}", self.connection.from);
        let result = match self.handle().await {
            Ok(response) => self.connection.reply(response).await,
            Err(e) => self.connection.reply_error(e).await,
        };
        permit.semaphore().add_permits(1);
        permit.forget();
        result
    }

    async fn handle(&mut self) -> ServerResult<ServerResponse> {
        if !self.config.allowed(&self.connection.from.ip()) {
            return Err(ServerError::new(
                StatusCode::UNAUTHORIZED,
                "Authentication failed",
            ));
        };

        let request = self.connection.request().await?;
        let auth = Authentication::from_request(&request)?;

        if !self.config.authenticate(auth) {
            return Err(ServerError::new(
                StatusCode::UNAUTHORIZED,
                "Authentication failed",
            ));
        };

        match self.router.get(&request) {
            Ok((req, args)) => match req {
                RequestType::REST(handler) => handler(request, args),
                RequestType::Resource(path) => ServerResponse::file(&path),
            },
            Err(e) => Err(e),
        }
    }
}
