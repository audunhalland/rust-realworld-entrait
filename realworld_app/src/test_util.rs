use axum::http::header::*;
use axum::http::StatusCode;
use axum::{body::Body, http::Request};
use bytes::Bytes;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tower::ServiceExt;

pub trait WithJsonBody<B: Serialize> {
    fn with_json_body(self, body: B) -> Request<Body>;
}

impl<B: Serialize> WithJsonBody<B> for http::request::Builder {
    fn with_json_body(self, body: B) -> Request<Body> {
        self.header(CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap()
    }
}

pub trait EmptyBody {
    fn empty_body(self) -> Request<Body>;
}

impl EmptyBody for http::request::Builder {
    fn empty_body(self) -> Request<Body> {
        self.body(Body::empty()).unwrap()
    }
}

pub async fn request(router: axum::Router, request: Request<Body>) -> (StatusCode, Bytes) {
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    match hyper::body::to_bytes(response.into_body()).await {
        Ok(bytes) => (status, bytes),
        Err(_) => panic!("error while fetching body"),
    }
}

pub async fn request_json<B: DeserializeOwned>(
    router: axum::Router,
    request: Request<Body>,
) -> Result<(StatusCode, B), (StatusCode, Bytes)> {
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    match hyper::body::to_bytes(response.into_body()).await {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(|body| (status, body))
            .map_err(|_| (status, bytes)),
        Err(_) => panic!("error while fetching body"),
    }
}
