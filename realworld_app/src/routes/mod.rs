mod article_routes;
mod profile_routes;
mod user_routes;

use crate::app::App;

use axum::routing::Router;
use entrait::Impl;

/// Axum API router for the real app.
pub fn api_router() -> axum::Router {
    Router::new().nest(
        "/api",
        Router::new()
            .merge(user_routes::UserRoutes::<Impl<App>>::router())
            .merge(profile_routes::ProfileRoutes::<Impl<App>>::router())
            .merge(article_routes::ArticleRoutes::<Impl<App>>::router()),
    )
}
