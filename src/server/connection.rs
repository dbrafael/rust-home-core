use super::ServerError;
use super::{
    response::{IntoResponse, ServerResponse},
    ServerResult,
};
use std::net::SocketAddr;
use tokio::{io::AsyncWriteExt, net::TcpStream};
use tokio_rustls::server::TlsStream;

pub struct Connection {
    pub from: SocketAddr,
    pub stream: TlsStream<TcpStream>,
}

impl Connection {
    pub async fn reply(&mut self, response: ServerResponse) -> ServerResult<()> {
        match self.stream.write_all(&response.into_bytes()).await {
            Ok(_) => Ok(()),
            Err(e) => Err(ServerError::new(
                http::StatusCode::INTERNAL_SERVER_ERROR,
                &format!("{}", e),
            )),
        }
    }

    pub async fn reply_error(&mut self, error: ServerError) -> ServerResult<()> {
        self.reply(ServerResponse::create(
            error.code,
            error.error.as_bytes().to_vec(),
        ))
        .await
    }
}
