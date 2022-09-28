#![cfg_attr(feature = "use-associated-future", feature(type_alias_impl_trait))]

mod app;
mod config;
mod routes;

use anyhow::Context;
use clap::Parser;
use entrait::Impl;
use std::sync::Arc;
use tower::ServiceBuilder;

#[cfg(test)]
mod test_util;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let config = config::Config::parse();
    let db = realworld_db::Db::init(&config.database_url).await?;

    // "link" the application by using the Impl type.
    // All trait implementations are for that type.
    let app = Impl::new(app::App {
        config: Arc::new(config),
        db,
    });

    let router = routes::api_router().layer(
        ServiceBuilder::new()
            // Inject the app into the axum context
            .layer(axum::extract::Extension(app))
            // Enables logging. Use `RUST_LOG=tower_http=debug`
            .layer(tower_http::trace::TraceLayer::new_for_http()),
    );

    axum::Server::bind(&"0.0.0.0:8080".parse()?)
        .serve(router.into_make_service())
        .await
        .context("error running HTTP server")?;

    Ok(())
}
