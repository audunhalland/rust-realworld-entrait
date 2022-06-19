use realworld_app::{app::App, config::Config};

use clap::Parser;
use implementation::Impl;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let config = Config::parse();
    let db = realworld_db::Db::init(&config.database_url).await?;

    realworld_app::serve(App {
        config: Arc::new(config),
        db: Impl::new(db),
    })
    .await?;

    Ok(())
}
