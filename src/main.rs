use http::{Method, Request, StatusCode};
use server::{
    IntoResponse, PathArgumentMap, RouteManager, ServerError, ServerRequest, ServerResponse,
    ServerResult,
};

mod server;

fn main() {
    let mut route_handler = RouteManager::default();
    route_handler
        .register("api/users", Method::GET, sum)
        .expect("Error registering route");
    route_handler
        .register("api/sum/[a]/[b]", Method::GET, sum)
        .expect("Error registering route");

    let (handler, vars) = route_handler.get("api/sum/123/876", Method::GET).unwrap();

    let request: ServerRequest = Request::builder().body(None).unwrap();
    println!("{:?}", vars);

    let _ = handler(request, vars);
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

    Ok(ServerResponse::create(StatusCode::OK, vec![]))
}
