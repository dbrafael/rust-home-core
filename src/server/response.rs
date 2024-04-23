use std::fs;

use http::{Response, StatusCode};

use super::{ServerError, ServerResult};

pub type ResponseBody = Vec<u8>;

pub type ServerResponse = Response<Option<ResponseBody>>;

trait BasicResponse {
    fn create_base(
        code: StatusCode,
        headers: Vec<(&str, &str)>,
        body: Option<ResponseBody>,
    ) -> ServerResponse {
        let mut response = Response::builder().status(code);
        for (k, v) in headers {
            response = response.header(k, v);
        }
        response.body(body).expect("Error building response")
    }
}

impl BasicResponse for ServerResponse {}

pub trait IntoResponse {
    fn create(code: StatusCode, body: ResponseBody) -> Self;
    fn file(filename: &str) -> ServerResult<ServerResponse>;
    fn download(filename: &str, body: ResponseBody) -> Self;
    fn json(body: &str) -> Self;
    fn into_bytes(self) -> Vec<u8>;
}

impl IntoResponse for ServerResponse {
    fn create(code: StatusCode, body: ResponseBody) -> Self {
        Self::create_base(code, vec![("Content-Type", "text/plain")], Some(body))
    }
    fn file(filename: &str) -> ServerResult<Self> {
        match fs::read(filename) {
            Ok(body) => Ok(ServerResponse::download(filename, body)),
            Err(e) => Err(ServerError::new(StatusCode::NOT_FOUND, &format!("{}", e))),
        }
    }
    fn download(filename: &str, body: ResponseBody) -> Self {
        Self::create_base(
            StatusCode::OK,
            vec![(
                "Content-Disposition",
                &format!("attachment; filename=\"{}\"", filename),
            )],
            Some(body),
        )
    }
    fn json(body: &str) -> Self {
        Self::create_base(
            StatusCode::OK,
            vec![("Content-Type", "application/json")],
            Some(body.into()),
        )
    }

    fn into_bytes(self) -> Vec<u8> {
        let head = format!("HTTP/1.1 {}", self.status());
        let headers = self
            .headers()
            .iter()
            .map(|(k, v)| format!("{}: {}\r\n", k, v.to_str().unwrap()))
            .collect::<String>();
        let body = self.body().clone().unwrap_or(vec![]);
        format!(
            "{}\r\n{}\r\n{}",
            head,
            headers,
            String::from_utf8_lossy(&body)
        )
        .into()
    }
}
