use axum::http::header::*;
use axum::http::StatusCode;
use axum::{body::Body, http::Request};
use bytes::Bytes;
use serde::Serialize;
use tower::ServiceExt;

pub fn build_json_post_request(uri: &str, body: &impl Serialize) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
        .body(Body::from(serde_json::to_vec(body).unwrap()))
        .unwrap()
}

pub async fn request_json(router: axum::Router, request: Request<Body>) -> (StatusCode, Bytes) {
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    match hyper::body::to_bytes(response.into_body()).await {
        Ok(bytes) => (status, bytes),
        Err(_) => panic!("error while fetching body"),
    }
}
