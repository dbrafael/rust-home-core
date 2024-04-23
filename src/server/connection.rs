use std::net::SocketAddr;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

use super::{response::IntoResponse, ServerResponse, ServerResult};
use super::{ServerError, ServerRequest};

pub struct Connection {
    pub from: SocketAddr,
    stream: TcpStream,
}

impl Connection {
    pub fn from_connection(stream: TcpStream, from: SocketAddr) -> Self {
        Self { from, stream }
    }

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

    pub async fn request(&mut self) -> ServerResult<ServerRequest> {
        let reader = BufReader::new(&mut self.stream);
        let mut lines = reader.lines();
        let mut builder = http::Request::builder();
        let has_body: bool;

        let first_line = match lines.next_line().await {
            Ok(line) => line,
            Err(e) => {
                return Err(ServerError::new(
                    http::StatusCode::BAD_REQUEST,
                    &format!("{}", e),
                ));
            }
        };

        let re_head = regex::Regex::new(r"^(GET|POST|PUT|DELETE) (.+) HTTP/1\.1$").unwrap();
        match re_head.captures(&first_line.unwrap()) {
            Some(caps) => {
                let method = match caps.get(1) {
                    Some(m) => m.as_str(),
                    None => return Err(ServerError::err("Invalid method")),
                };
                let uri = match caps.get(2) {
                    Some(u) => u.as_str(),
                    None => return Err(ServerError::err("Invalid URI")),
                };
                builder = builder.method(method).uri(uri);
                has_body = builder.method_ref().unwrap() == "POST"
                    || builder.method_ref().unwrap() == "PUT";
            }
            None => return Err(ServerError::err("Invalid request")),
        };

        let re_header = regex::Regex::new(r"^([\w-]+): (.+)$").unwrap();
        loop {
            let line = match lines.next_line().await {
                Ok(Some(line)) => line,
                Ok(None) => {
                    if has_body {
                        return Err(ServerError::err("No body"));
                    }
                    break;
                }
                _ => return Err(ServerError::err("Error reading header")),
            };
            if line.is_empty() {
                break;
            }
            match re_header.captures(&line) {
                Some(caps) => {
                    let key = match caps.get(1) {
                        Some(k) => k.as_str(),
                        None => return Err(ServerError::err("Invalid key")),
                    };
                    let value = match caps.get(2) {
                        Some(v) => v.as_str(),
                        None => return Err(ServerError::err("Invalid value")),
                    };
                    builder = builder.header(key, value);
                }
                None => return Err(ServerError::err("Invalid header")),
            }
        }
        let mut body = String::new();
        if has_body {
            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => body.push_str(&line),
                    Ok(None) => break,
                    _ => return Err(ServerError::err("Error reading body")),
                };
            }
        }

        Ok(builder
            .body(if has_body { Some(body) } else { None })
            .unwrap())
    }
}
