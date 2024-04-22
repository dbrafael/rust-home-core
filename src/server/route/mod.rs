pub mod manager;
pub mod parser;
pub mod route;

use self::parser::PathArgumentMap;

use super::{request::ServerRequest, response::ServerResponse, ServerResult};

pub type RequestHandler = fn(ServerRequest, PathArgumentMap) -> ServerResult<ServerResponse>;
