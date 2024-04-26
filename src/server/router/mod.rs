pub mod parser;
pub mod router;

use router::PathArguments;

use super::request::ServerRequest;
use super::response::ServerResponse;
use super::ServerResult;

pub type RequestHandler = fn(ServerRequest, PathArguments) -> ServerResult<ServerResponse>;
