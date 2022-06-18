use crate::app::App;
use crate::error::Error;

use entrait::unimock_test::*;
use sqlx::error::DatabaseError;
use sqlx::PgPool;

pub mod user_db;

#[entrait(pub GetPgPool)]
fn get_pg_pool(_: &App) -> &PgPool {
    unimplemented!()
}

impl GetPgPool for sqlx::PgPool {
    fn get_pg_pool(&self) -> &PgPool {
        self
    }
}

trait DbResultExt<T> {
    fn on_constraint(
        self,
        name: &str,
        f: impl FnOnce(Box<dyn DatabaseError>) -> Error,
    ) -> Result<T, Error>;
}

impl<T, E> DbResultExt<T> for Result<T, E>
where
    E: Into<Error>,
{
    fn on_constraint(
        self,
        name: &str,
        map_err: impl FnOnce(Box<dyn DatabaseError>) -> Error,
    ) -> Result<T, Error> {
        self.map_err(|e| match e.into() {
            Error::Sqlx(sqlx::Error::Database(dbe)) if dbe.constraint() == Some(name) => {
                map_err(dbe)
            }
            e => e,
        })
    }
}

#[cfg(test)]
async fn create_test_db() -> sqlx::PgPool {
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

    sqlx::migrate!()
        .run(&pg_pool)
        .await
        .expect("Failed to migrate");

    pg_pool
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
