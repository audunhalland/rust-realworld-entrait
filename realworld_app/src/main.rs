use realworld_app::{app::App, config::Config, routes};

use anyhow::Context;
use clap::Parser;
use implementation::Impl;
use std::sync::Arc;
use tower::ServiceBuilder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let config = Config::parse();
    let db = realworld_db::Db::init(&config.database_url).await?;

    // "link" the application by using the Impl type.
    // All trait implementations are for that type.
    let app = Impl::new(App {
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
