use super::GetPgPool;
use crate::error::AppResult;
use crate::App;

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

pub struct PasswordHash(pub String);

#[entrait(InsertUser for App, async_trait=true, unimock=test)]
async fn insert_user(
    deps: &impl GetPgPool,
    username: String,
    email: String,
    password_hash: PasswordHash,
) -> AppResult<DbUser> {
    let id = sqlx::query_scalar!(
        r#"INSERT INTO app.user (username, email, password_hash) VALUES ($1, $2, $3) RETURNING id"#,
        username,
        email,
        password_hash.0
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

#[entrait(FetchUserAndPasswordHashByEmail for App, async_trait=true, unimock=test)]
async fn fetch_user_and_password_hash_by_email(
    deps: &impl GetPgPool,
    email: String,
) -> AppResult<Option<(DbUser, PasswordHash)>> {
    let record = sqlx::query!(
        r#"SELECT id, email, username, password_hash, bio, image FROM app.user WHERE email = $1"#,
        email
    )
    .fetch_optional(deps.get_pg_pool())
    .await?;

    Ok(record.map(|record| {
        (
            DbUser {
                id: record.id,
                username: record.username,
                email: record.email,
                bio: record.bio,
                image: record.image,
            },
            PasswordHash(record.password_hash),
        )
    }))
}

#[entrait(FetchUserById for App, async_trait=true, unimock=test)]
async fn fetch_user_by_id(deps: &impl GetPgPool, id: Uuid) -> AppResult<DbUser> {
    let db_user = sqlx::query_as!(
        DbUser,
        r#"SELECT id, email, username, bio, image FROM app.user WHERE id = $1"#,
        id
    )
    .fetch_one(deps.get_pg_pool())
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
            PasswordHash("baz".to_string()),
        )
        .await
        .unwrap();

        assert_eq!("foo", created_user.username);
        assert_eq!("bar", created_user.email);

        let fetched_user = fetch_user_by_id(&pool, created_user.id).await.unwrap();

        assert_eq!(created_user, fetched_user);
    }
}
