use crate::App;
use entrait::*;
use sqlx::PgPool;

pub mod user_db;

#[entrait(GetPgPool for App)]
fn get_pg_pool(_: &App) -> &PgPool {
    unimplemented!()
}

impl GetPgPool for sqlx::PgPool {
    fn get_pg_pool(&self) -> &PgPool {
        self
    }
}

#[cfg(test)]
async fn create_test_db() -> sqlx::PgPool {
    use sqlx::Connection;

    let db_name = format!("test_db_{}", std::thread::current().name().unwrap());
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
