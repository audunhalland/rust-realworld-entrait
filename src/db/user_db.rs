use super::GetPgPool;
use crate::error::Result;

use entrait::*;
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DbUser {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub bio: String,
    pub image: Option<String>,
}

#[entrait(InsertUser for crate::App, async_trait=true, unimock=test)]
async fn insert_user(
    deps: &impl GetPgPool,
    username: String,
    email: String,
    password_hash: String,
) -> Result<DbUser> {
    let id = sqlx::query_scalar!(
        r#"INSERT INTO app.user (username, email, password_hash) VALUES ($1, $2, $3) RETURNING id"#,
        username,
        email,
        password_hash
    )
    .fetch_one(deps.get_pg_pool())
    .await?;

    Ok(DbUser {
        id,
        username,
        email,
        bio: "".to_string(),
        image: None,
    })
}

#[entrait(FetchUserById for crate::App, async_trait=true, unimock=test)]
async fn fetch_user_by_id(deps: &impl GetPgPool, id: Uuid) -> Result<DbUser> {
    let db_user = sqlx::query_as!(
        DbUser,
        r#"SELECT id, email, username, bio, image FROM app.user WHERE id = $1"#,
        id
    )
    .fetch_one(deps.get_pg_pool())
    .await?;

    Ok(db_user)
}

#[entrait(FetchUserByEmail for crate::App, async_trait=true, unimock=test)]
async fn fetch_user_by_email(deps: &impl GetPgPool, email: String) -> Result<Option<DbUser>> {
    let db_user = sqlx::query_as!(
        DbUser,
        r#"SELECT id, email, username, bio, image FROM app.user WHERE email = $1"#,
        email
    )
    .fetch_optional(deps.get_pg_pool())
    .await?;

    Ok(db_user)
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;

    #[tokio::test]
    async fn should_insert_then_fetch_user() {
        let pool = create_test_db().await;

        let created_user = insert_user(
            &pool,
            "foo".to_string(),
            "bar".to_string(),
            "baz".to_string(),
        )
        .await
        .unwrap();

        assert_eq!("foo", created_user.username);
        assert_eq!("bar", created_user.email);

        let fetched_user = fetch_user_by_id(&pool, created_user.id).await.unwrap();

        assert_eq!(created_user, fetched_user);
    }
}
