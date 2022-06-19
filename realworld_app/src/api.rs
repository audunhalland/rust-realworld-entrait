mod article_api;
mod user_api;

use crate::app::App;

use axum::routing::Router;
use implementation::Impl;

/// Axum API router for the real app.
pub fn api_router() -> axum::Router {
    Router::new().nest(
        "/api",
        Router::new()
            .merge(user_api::UserApi::<Impl<App>>::router())
            .merge(article_api::ArticleApi::<Impl<App>>::router()),
    )
}
