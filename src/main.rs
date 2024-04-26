use std::{
    net::{IpAddr, Ipv4Addr},
    sync::{Arc, Mutex},
};

use http::StatusCode;
use server::{
    auth::Authentication,
    request::ServerRequest,
    response::{IntoResponse, ServerResponse},
    router::router::{PathArguments, RouterBuilder},
    server::{Server, ServerConfig},
    ServerError, ServerResult,
};

use crate::server::auth::AuthBuilder;

pub mod common;
mod server;

fn main() {}
