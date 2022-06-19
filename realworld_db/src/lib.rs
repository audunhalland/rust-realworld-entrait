use realworld_core::error::RwError;

use anyhow::Context;
use entrait::unimock::*;
use sqlx::error::DatabaseError;
use sqlx::PgPool;

pub mod user_db;

#[derive(Clone)]
pub struct Db {
    pub pg_pool: PgPool,
}

impl Db {
    pub async fn init(url: &str) -> anyhow::Result<Self> {
        let pg_pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(50)
            .connect(url)
            .await
            .context("could not connect to database_url")?;

        sqlx::migrate!("../migrations").run(&pg_pool).await?;

        Ok(Db { pg_pool })
    }
}

/// Export an entrait module
pub trait DbModule:
    user_db::FindUserByEmail + user_db::FindUserById + user_db::InsertUser + user_db::UpdateUser
{
}

impl DbModule for ::implementation::Impl<Db> {}
impl DbModule for unimock::Unimock {}

#[entrait(pub GetPgPool)]
fn get_pg_pool(db: &Db) -> &PgPool {
    &db.pg_pool
}

impl GetPgPool for Db {
    fn get_pg_pool(&self) -> &PgPool {
        &self.pg_pool
    }
}

trait DbResultExt<T> {
    fn on_constraint(
        self,
        name: &str,
        f: impl FnOnce(Box<dyn DatabaseError>) -> RwError,
    ) -> Result<T, RwError>;
}

impl<T, E> DbResultExt<T> for Result<T, E>
where
    E: Into<RwError>,
{
    fn on_constraint(
        self,
        name: &str,
        map_err: impl FnOnce(Box<dyn DatabaseError>) -> RwError,
    ) -> Result<T, RwError> {
        self.map_err(|e| match e.into() {
            RwError::Sqlx(sqlx::Error::Database(dbe)) if dbe.constraint() == Some(name) => {
                map_err(dbe)
            }
            e => e,
        })
    }
}

#[cfg(test)]
async fn create_test_db() -> implementation::Impl<Db> {
    use sha2::Digest;
    use sqlx::Connection;

    let mut hasher = sha2::Sha256::new();
    hasher.update(std::thread::current().name().unwrap().as_bytes());
    let thread_hash = hex::encode(hasher.finalize());
    let db_name = &thread_hash[0..24];

    let mut url = database_server_url();
    let mut connection = sqlx::PgConnection::connect(url.as_str()).await.unwrap();

    sqlx::query(&format!(r#"DROP DATABASE IF EXISTS "{}""#, db_name))
        .execute(&mut connection)
        .await
        .expect("failed to drop");

    sqlx::query(&format!(r#"CREATE DATABASE "{}""#, db_name))
        .execute(&mut connection)
        .await
        .expect("failed creating test database");

    url.set_path(&db_name);

    let pg_pool = sqlx::PgPool::connect(url.as_str())
        .await
        .expect("Failed to connect to database");

    sqlx::migrate!("../migrations")
        .run(&pg_pool)
        .await
        .expect("Failed to migrate");

    implementation::Impl::new(Db { pg_pool })
}

#[cfg(test)]
fn database_server_url() -> url::Url {
    // (re)load the .env file
    dotenv::dotenv().ok();

    let mut url: url::Url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set")
        .parse()
        .expect("malformed DATABASE_URL");

    if let Ok(mut path) = url.path_segments_mut() {
        path.clear();
    }

    url
}
