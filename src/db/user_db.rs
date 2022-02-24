use super::GetPgPool;
use crate::error::Result;

use entrait::*;
use uuid::Uuid;

#[derive(Debug, Eq, PartialEq)]
pub struct DbUser {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub bio: String,
    pub image: Option<String>,
}

#[entrait(CreateUser for crate::App)]
async fn create_user<A>(
    a: &A,
    username: String,
    email: String,
    password_hash: String,
) -> Result<DbUser>
where
    A: GetPgPool,
{
    let id = sqlx::query_scalar!(
        r#"INSERT INTO app.user (username, email, password_hash) VALUES ($1, $2, $3) RETURNING id"#,
        username,
        email,
        password_hash
    )
    .fetch_one(a.get_pg_pool())
    .await?;

    Ok(DbUser {
        id,
        username,
        email,
        bio: "".to_string(),
        image: None,
    })
}

#[entrait(GetUser for crate::App)]
async fn get_user<A>(a: &A, id: Uuid) -> Result<DbUser>
where
    A: GetPgPool,
{
    let db_user = sqlx::query_as!(
        DbUser,
        r#"SELECT id, email, username, bio, image FROM app.user WHERE id = $1"#,
        id
    )
    .fetch_one(a.get_pg_pool())
    .await?;

    Ok(db_user)
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;

    #[tokio::test]
    async fn should_create_then_retrieve_user() {
        let pool = create_test_db().await;

        let created_user = create_user(
            &pool,
            "foo".to_string(),
            "bar".to_string(),
            "baz".to_string(),
        )
        .await
        .unwrap();

        assert_eq!("foo", created_user.username);
        assert_eq!("bar", created_user.email);

        let fetched_user = get_user(&pool, created_user.id).await.unwrap();

        assert_eq!(created_user, fetched_user);
    }
}
