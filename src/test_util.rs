use axum::http::header::*;
use axum::http::StatusCode;
use axum::{body::Body, http::Request};
use bytes::Bytes;
use hyper::Response;
use serde::Deserialize;
use serde::Serialize;

pub fn build_json_post_request(uri: &str, body: &impl Serialize) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/api/users")
        .header(CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
        .body(Body::from(serde_json::to_vec(body).unwrap()))
        .unwrap()
}

pub async fn fetch_json_body<B>(response: Response<B>) -> (StatusCode, Bytes)
where
    B: hyper::body::HttpBody + std::fmt::Debug,
{
    let status = response.status();
    match hyper::body::to_bytes(response.into_body()).await {
        Ok(bytes) => (status, bytes),
        Err(e) => panic!("error while fetching body"),
    }
}
