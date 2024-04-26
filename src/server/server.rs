use std::{
    fs,
    io::BufReader,
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use http::StatusCode;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio::{
    net::TcpListener,
    sync::{OwnedSemaphorePermit, Semaphore},
    task::JoinHandle,
};
use tokio_rustls::TlsAcceptor;

use super::{
    auth::{AuthManager, Authentication},
    connection::Connection,
    request::ServerRequest,
    response::ServerResponse,
    router::router::Router,
    ServerError, ServerResult,
};

#[derive(Clone)]
pub struct ServerConfig {
    pub server_address: SocketAddr,
    pub max_workers: usize,
    pub ss_dir: &'static str,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server_address: SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8080),
            max_workers: 5,
            ss_dir: "/tmp/ssl/",
        }
    }
}

pub struct Server {
    config: ServerConfig,
    tls: TlsAcceptor,
    routes: Arc<Router>,
    auth: Arc<AuthManager>,
    worker_pool: Arc<Semaphore>,
}

impl Server {
    pub fn new(config: ServerConfig, routes: Router, auth: AuthManager) -> ServerResult<Server> {
        let certs = load_certs(&config.ss_dir)?;
        let pk = load_pk(&config.ss_dir)?;

        let mut tls_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, pk)
            .map_err(|e| ServerError::err(&format!("Error creating tls config: {}", e)))?;
        tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        let tls = TlsAcceptor::from(Arc::new(tls_config));

        Ok(Self {
            worker_pool: Arc::new(Semaphore::new(config.max_workers)),
            config,
            auth: Arc::new(auth),
            routes: Arc::new(routes),
            tls,
        })
    }

    pub fn run(self) -> JoinHandle<ServerResult<()>> {
        tokio::spawn(async move {
            let listener = TcpListener::bind(self.config.server_address)
                .await
                .map_err(|e| ServerError::err(&format!("Error binding to address: {e}")))?;
            println!("Server listening on: {}", self.config.server_address);
            loop {
                let con = self.next_connection(&listener).await?;
                let permit = self
                    .worker_pool
                    .clone()
                    .acquire_owned()
                    .await
                    .map_err(|e| {
                        ServerError::err(&format!("Error getting permit for worker: {e}"))
                    })?;
                ServerWorker::spawn(self.auth.clone(), self.routes.clone(), con, permit);
            }
        })
    }

    async fn next_connection(&self, listener: &TcpListener) -> ServerResult<Connection> {
        let (stream, addr) = listener
            .accept()
            .await
            .map_err(|e| ServerError::err(&format!("Error accepting connection: {e}")))?;
        let tls_stream = self
            .tls
            .accept(stream)
            .await
            .map_err(|e| ServerError::err(&format!("Error converting to TLS: {e}")))?;
        let connection = Connection {
            from: addr,
            stream: tls_stream,
        };
        Ok(connection)
    }
}

struct ServerWorker {
    auth: Arc<AuthManager>,
    routes: Arc<Router>,
    connection: Connection,
}

impl ServerWorker {
    pub fn spawn(
        auth: Arc<AuthManager>,
        routes: Arc<Router>,
        connection: Connection,
        permit: OwnedSemaphorePermit,
    ) -> JoinHandle<ServerResult<()>> {
        let mut worker = Self {
            auth,
            routes,
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
        if !self.auth.allows(self.connection.from.ip()) {
            return Err(ServerError::new(
                StatusCode::UNAUTHORIZED,
                "Authentication failed",
            ));
        };

        let request = ServerRequest::from_connection(&mut self.connection).await?;
        let auth = Authentication::from_request(&request)?;

        if !self.auth.authenticate(&auth) {
            return Err(ServerError::new(
                StatusCode::UNAUTHORIZED,
                "Authentication failed",
            ));
        };

        self.routes.resolve(request)
    }
}

fn load_certs(path: &str) -> ServerResult<Vec<CertificateDer<'static>>> {
    let certfile = fs::File::open(&format!("{}/cert.pem", path))
        .map_err(|e| ServerError::err(&format!("Error opening cert file: {}", e)))?;
    let mut reader = BufReader::new(certfile);
    Ok(rustls_pemfile::certs(&mut reader)
        .filter(|x| x.is_ok())
        .map(|x| x.unwrap())
        .collect())
}

fn load_pk(path: &str) -> ServerResult<PrivateKeyDer<'static>> {
    let pkfile = fs::File::open(&format!("{}/key.pem", path))
        .map_err(|e| ServerError::err(&format!("Error opening pk file: {}", e)))?;
    let mut reader = BufReader::new(pkfile);
    match rustls_pemfile::private_key(&mut reader) {
        Ok(Some(pk)) => Ok(pk),
        Ok(None) => Err(ServerError::err("No private key found")),
        Err(e) => Err(ServerError::err(&format!("Error reading pk file: {}", e))),
    }
}
