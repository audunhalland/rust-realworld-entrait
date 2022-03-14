mod user_api;

pub fn api_router() -> axum::Router {
    // This is the order that the modules were authored in.
    user_api::router()
}
