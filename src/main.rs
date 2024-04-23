use std::net::{IpAddr, Ipv4Addr};

use http::StatusCode;
use server::{
    HTTPServer, IntoResponse, PathArgumentMap, ServerConfig, ServerError, ServerRequest,
    ServerResponse, ServerResult, ServerRoute,
};

pub mod common;
mod server;

#[tokio::main]
async fn main() -> ServerResult<()> {
    let mut config = ServerConfig::default();
    config.allow_address(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    config.allow_user("test", "123");
    let server = HTTPServer::new(
        config,
        vec![
            ServerRoute::REST("/sum/[a]/[b]/", http::Method::GET, sum),
            ServerRoute::Resource("favicon.ico", "/tmp/test"),
        ],
    )?;
    let handler = server.start().await?;
    let _ = handler.await;
    Ok(())
}

fn sum(_: ServerRequest, args: PathArgumentMap) -> ServerResult<ServerResponse> {
    let a = match args.get("a") {
        Some(n) => n
            .parse::<i32>()
            .map_err(|_| ServerError::err("Not a number"))?,
        _ => return Err(ServerError::err("Missing argument")),
    };
    let b = match args.get("b") {
        Some(n) => n
            .parse::<i32>()
            .map_err(|_| ServerError::err("Not a number"))?,
        _ => return Err(ServerError::err("Missing argument")),
    };
    println!("{} + {} = {}", a, b, a + b);

    Ok(ServerResponse::create(StatusCode::OK, vec![(a + b) as u8]))
}
