#![cfg_attr(feature = "use-associated-future", feature(generic_associated_types))]
#![cfg_attr(feature = "use-associated-future", feature(type_alias_impl_trait))]

use realworld_domain::error::RwError;

use anyhow::Context;
use entrait::entrait_export as entrait;
use sqlx::error::DatabaseError;
use sqlx::PgPool;

pub mod article;
pub mod comment;
pub mod user;

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

#[entrait(pub GetDb)]
fn get_db(db: &Db) -> &Db {
    db
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
impl realworld_domain::user::repo::DelegateUserRepo<Self> for Db {
    type Target = user::repo::Repo;
}

#[cfg(test)]
impl realworld_domain::article::repo::DelegateArticleRepo<Self> for Db {
    type Target = article::repo::Repo;
}

#[cfg(test)]
impl realworld_domain::comment::repo::DelegateCommentRepo<Self> for Db {
    type Target = comment::repo::Repo;
}

#[cfg(test)]
async fn create_test_db() -> entrait::Impl<Db> {
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

    entrait::Impl::new(Db { pg_pool })
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
