use http::Request;

use crate::server::{ServerError, ServerResult};

pub type RequestBody = Option<String>;
pub type ServerRequest = Request<RequestBody>;

pub trait RequestArgs<T: Sized = Self> {
    fn get_arg(&self, key: &str) -> ServerResult<&str>;
}

impl RequestArgs for ServerRequest {
    fn get_arg(&self, key: &str) -> ServerResult<&str> {
        match self.uri().query() {
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
}
