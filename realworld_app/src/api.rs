mod user_api;

use crate::app::App;

use implementation::Impl;

/// Axum API router for the real app.
pub fn api_router() -> axum::Router {
    // This is the order that the modules were authored in.
    user_api::UserApi::<Impl<App>>::router()
}
