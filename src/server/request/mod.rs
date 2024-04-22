use http::Request;
use std::io::{Error, ErrorKind, Result};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::TcpStream,
};

pub type RequestBody = Option<String>;
pub type ServerRequest = Request<RequestBody>;

pub trait RequestArgs<T: Sized = Self> {
    fn get_arg(&self, key: &str) -> Result<Option<&str>>;
}

impl RequestArgs for ServerRequest {
    fn get_arg(&self, key: &str) -> Result<Option<&str>> {
        match self.uri().query() {
            Some(query) => {
                let re = regex::Regex::new(&format!(r"{}=([^&]+)", key)).unwrap();
                match re.captures(query) {
                    Some(caps) => Ok(Some(caps.get(1).unwrap().as_str())),
                    None => Ok(None),
                }
            }
            None => Err(Error::new(ErrorKind::InvalidInput, "No query string")),
        }
    }
}

pub async fn request_from_stream(stream: &mut TcpStream) -> Result<ServerRequest> {
    let reader = BufReader::new(stream);
    let mut lines = reader.lines();
    let mut builder = http::Request::builder();
    let has_body: bool;

    let first_line = match lines.next_line().await {
        Ok(line) => line,
        Err(e) => return Err(e),
    };
    let re_head = regex::Regex::new(r"^(GET|POST|PUT|DELETE) (.+) HTTP/1\.1$").unwrap();
    match re_head.captures(&first_line.unwrap()) {
        Some(caps) => {
            let method = match caps.get(1) {
                Some(m) => m.as_str(),
                None => return Err(Error::new(ErrorKind::InvalidInput, "Invalid method")),
            };
            let uri = match caps.get(2) {
                Some(u) => u.as_str(),
                None => return Err(Error::new(ErrorKind::InvalidInput, "Invalid URI")),
            };
            builder = builder.method(method).uri(uri);
            has_body =
                builder.method_ref().unwrap() == "POST" || builder.method_ref().unwrap() == "PUT";
        }
        None => return Err(Error::new(ErrorKind::InvalidInput, "Invalid request")),
    };

    let re_header = regex::Regex::new(r"^([\w-]+): (.+)$").unwrap();
    loop {
        let line = match lines.next_line().await {
            Ok(Some(line)) => line,
            Ok(None) => {
                if has_body {
                    return Err(Error::new(ErrorKind::InvalidInput, "No body"));
                }
                break;
            }
            _ => return Err(Error::new(ErrorKind::InvalidInput, "Error reading header")),
        };
        if line.is_empty() {
            break;
        }
        match re_header.captures(&line) {
            Some(caps) => {
                let key = match caps.get(1) {
                    Some(k) => k.as_str(),
                    None => return Err(Error::new(ErrorKind::InvalidInput, "Invalid key")),
                };
                let value = match caps.get(2) {
                    Some(v) => v.as_str(),
                    None => return Err(Error::new(ErrorKind::InvalidInput, "Invalid value")),
                };
                builder = builder.header(key, value);
            }
            None => return Err(Error::new(ErrorKind::InvalidInput, "Invalid header")),
        }
    }
    let mut body = String::new();
    if has_body {
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => body.push_str(&line),
                Ok(None) => break,
                _ => return Err(Error::new(ErrorKind::InvalidInput, "Error reading header")),
            };
        }
    }
    Ok(builder
        .body(if has_body { Some(body) } else { None })
        .unwrap())
}
