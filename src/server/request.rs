use crate::server::{ServerError, ServerResult};
use http::{Method, Request};
use tokio::io::{AsyncBufReadExt, BufReader};

use super::connection::Connection;

pub type RequestBody = Option<String>;

#[derive(Debug, Clone)]
pub struct ServerRequest(Request<RequestBody>);

impl ServerRequest {
    pub fn method(&self) -> &Method {
        self.0.method()
    }

    pub fn path(&self) -> &str {
        self.0.uri().path()
    }

    pub fn body_str(&self) -> String {
        self.0.body().clone().unwrap().into()
    }

    pub fn query_argument(&self, key: &str) -> ServerResult<&str> {
        match self.0.uri().query() {
            Some(query) => {
                let re = regex::Regex::new(&format!(r"{}=([^&]+)", key)).unwrap();
                match re.captures(query) {
                    Some(caps) => Ok(caps.get(1).unwrap().as_str()),
                    None => Err(ServerError::err("No match")),
                }
            }
            None => Err(ServerError::err("No query string")),
        }
    }

    pub async fn from_connection(connection: &mut Connection) -> ServerResult<Self> {
        let reader = BufReader::new(&mut connection.stream);
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

        let request = builder
            .body(if has_body { Some(body) } else { None })
            .unwrap();

        Ok(Self(request))
    }
}
