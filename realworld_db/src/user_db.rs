use crate::DbResultExt;
use crate::GetPgPool;
use realworld_core::error::{RwError, RwResult};
use realworld_core::{PasswordHash, UserId};

use entrait::unimock::*;
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DbUser {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub bio: String,
    pub image: Option<String>,
}

#[derive(Clone, Default)]
pub struct DbUserUpdate {
    pub email: Option<String>,
    pub username: Option<String>,
    pub password_hash: Option<PasswordHash>,
    pub bio: Option<String>,
    pub image: Option<String>,
}

#[entrait(pub InsertUser, async_trait=true)]
async fn insert_user(
    deps: &impl GetPgPool,
    username: String,
    email: String,
    password_hash: PasswordHash,
) -> RwResult<DbUser> {
    let id = sqlx::query_scalar!(
        r#"INSERT INTO app.user (username, email, password_hash) VALUES ($1, $2, $3) RETURNING id"#,
        username,
        email,
        password_hash.0
    )
    .fetch_one(deps.get_pg_pool())
    .await
    .on_constraint("user_username_key", |_| RwError::UsernameTaken)
    .on_constraint("user_email_key", |_| RwError::EmailTaken)?;

    Ok(DbUser {
        id,
        username,
        email,
        bio: "".to_string(),
        image: None,
    })
}

#[entrait(pub FindUserById, async_trait = true)]
async fn find_user_by_id(
    deps: &impl GetPgPool,
    id: UserId,
) -> RwResult<Option<(DbUser, PasswordHash)>> {
    let record = sqlx::query!(
        r#"SELECT id, email, username, password_hash, bio, image FROM app.user WHERE id = $1"#,
        id.0
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

#[entrait(pub FindUserByEmail, async_trait = true)]
async fn find_user_by_email(
    deps: &impl GetPgPool,
    email: String,
) -> RwResult<Option<(DbUser, PasswordHash)>> {
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

#[entrait(pub UpdateUser, async_trait=true)]
async fn update_user(deps: &impl GetPgPool, id: UserId, update: DbUserUpdate) -> RwResult<DbUser> {
    let user = sqlx::query!(
        // language=PostgreSQL
        r#"
        UPDATE app.user SET
            email = COALESCE($1, email),
            username = COALESCE($2, username),
            password_hash = COALESCE($3, password_hash),
            bio = COALESCE($4, bio),
            image = COALESCE($5, image)
        WHERE id = $6
        RETURNING email, username, bio, image
        "#,
        update.email,
        update.username,
        update.password_hash.map(|hash| hash.0),
        update.bio,
        update.image,
        id.clone().0
    )
    .fetch_one(deps.get_pg_pool())
    .await
    .on_constraint("user_username_key", |_| RwError::UsernameTaken)
    .on_constraint("user_email_key", |_| RwError::EmailTaken)?;

    Ok(DbUser {
        id: id.0,
        username: user.username,
        email: user.email,
        bio: user.bio,
        image: user.image,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create_test_db;
    use crate::Db;

    use assert_matches::*;

    struct TestNewUser {
        username: &'static str,
        email: &'static str,
        password_hash: &'static str,
    }

    impl Default for TestNewUser {
        fn default() -> Self {
            Self {
                username: "username",
                email: "email",
                password_hash: "hash",
            }
        }
    }

    fn other_user() -> TestNewUser {
        TestNewUser {
            username: "username2",
            email: "email2",
            password_hash: "hash2",
        }
    }

    async fn insert_test_user(db: &Db, user: TestNewUser) -> RwResult<DbUser> {
        insert_user(
            db,
            user.username.to_string(),
            user.email.to_string(),
            PasswordHash(user.password_hash.to_string()),
        )
        .await
    }

    #[tokio::test]
    async fn should_insert_then_fetch_user() {
        let db = create_test_db().await;
        let created_user = insert_test_user(&db, TestNewUser::default()).await.unwrap();

        assert_eq!("username", created_user.username);
        assert_eq!("email", created_user.email);

        let (fetched_user, _) = db
            .find_user_by_id(UserId(created_user.id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(created_user, fetched_user);
    }

    #[tokio::test]
    async fn should_fail_to_create_two_users_with_the_same_username() {
        let db = create_test_db().await;
        insert_test_user(&db, TestNewUser::default()).await.unwrap();

        let error = insert_test_user(&db, TestNewUser::default())
            .await
            .expect_err("should error");

        assert_matches!(error, RwError::UsernameTaken);
    }

    #[tokio::test]
    async fn should_fail_to_create_two_users_with_the_same_email() {
        let db = create_test_db().await;
        insert_test_user(&db, TestNewUser::default()).await.unwrap();

        let error = insert_test_user(
            &db,
            TestNewUser {
                username: "newusername",
                ..TestNewUser::default()
            },
        )
        .await
        .expect_err("should error");

        assert_matches!(error, RwError::EmailTaken);
    }

    #[tokio::test]
    async fn should_update_user() {
        let db = create_test_db().await;
        let created_user = insert_test_user(&db, TestNewUser::default()).await.unwrap();

        let updated_user = db
            .update_user(
                UserId(created_user.id),
                DbUserUpdate {
                    email: Some("newmail".to_string()),
                    username: Some("newname".to_string()),
                    password_hash: Some(PasswordHash("newhash".to_string())),
                    bio: Some("newbio".to_string()),
                    image: Some("newimage".to_string()),
                },
            )
            .await
            .unwrap();

        assert_eq!(created_user.id, updated_user.id);
        assert_eq!("newmail", updated_user.email);
        assert_eq!("newname", updated_user.username);
        assert_eq!("newbio", updated_user.bio);
        assert_eq!(Some("newimage"), updated_user.image.as_deref());
    }

    #[tokio::test]
    async fn should_fail_to_update_user_to_taken_username() {
        let db = create_test_db().await;
        insert_test_user(&db, TestNewUser::default()).await.unwrap();
        let user = insert_test_user(&db, other_user()).await.unwrap();

        let error = db
            .update_user(
                UserId(user.id),
                DbUserUpdate {
                    username: Some("username".to_string()),
                    ..DbUserUpdate::default()
                },
            )
            .await
            .expect_err("should error");

        assert_matches!(error, RwError::UsernameTaken);
    }

    #[tokio::test]
    async fn should_fail_to_update_user_to_taken_email() {
        let db = create_test_db().await;
        insert_test_user(&db, TestNewUser::default()).await.unwrap();
        let user = insert_test_user(&db, other_user()).await.unwrap();

        let error = db
            .update_user(
                UserId(user.id),
                DbUserUpdate {
                    email: Some("email".to_string()),
                    ..DbUserUpdate::default()
                },
            )
            .await
            .expect_err("should error");

        assert_matches!(error, RwError::EmailTaken);
    }
}
