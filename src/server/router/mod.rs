mod endpoint;
mod parser;
mod router;

pub use parser::PathArgumentMap;
pub use router::Router;

use super::{ServerRequest, ServerResponse, ServerResult};

pub type RequestHandler = fn(ServerRequest, PathArgumentMap) -> ServerResult<ServerResponse>;
