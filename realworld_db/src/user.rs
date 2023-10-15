use crate::DbResultExt;
use crate::GetDb;

use realworld_domain::error::{RwError, RwResult};
use realworld_domain::user::email::Email;
use realworld_domain::user::password::PasswordHash;
use realworld_domain::user::repo::*;
use realworld_domain::user::UserId;

use entrait::*;

pub struct PgUserRepo;

#[entrait]
impl realworld_domain::user::repo::UserRepoImpl for PgUserRepo {
    pub async fn insert_user(
        deps: &impl GetDb,
        username: &str,
        email: &Email,
        password_hash: PasswordHash,
    ) -> RwResult<(User, Credentials)> {
        let mut tx = deps.get_db().pg_pool.begin().await?;

        let id = sqlx::query_scalar!(
            r#"INSERT INTO app.user (username, email, password_hash) VALUES ($1, $2, $3) RETURNING user_id"#,
            username,
            email.as_ref(),
            password_hash.0
        )
        .fetch_one(&mut *tx)
        .await
        .on_constraint("user_username_key", |_| RwError::UsernameTaken)
        .on_constraint("user_email_key", |_| RwError::EmailTaken)?;

        tx.commit().await?;

        Ok((
            User {
                user_id: UserId(id),
                username: username.to_string(),
                bio: "".to_string(),
                image: None,
            },
            Credentials {
                email: email.clone(),
                password_hash,
            },
        ))
    }

    pub async fn find_user_credentials_by_id(
        deps: &impl GetDb,
        UserId(user_id): UserId,
    ) -> RwResult<Option<(User, Credentials)>> {
        let record = sqlx::query!(
            r#"SELECT user_id, email, username, password_hash, bio, image FROM app.user WHERE user_id = $1"#,
            user_id
        )
        .fetch_optional(&deps.get_db().pg_pool)
        .await?;

        Ok(record.map(|record| {
            (
                User {
                    user_id: UserId(record.user_id),
                    username: record.username,
                    bio: record.bio,
                    image: record.image,
                },
                Credentials {
                    email: Email::valid(record.email),
                    password_hash: record.password_hash.into(),
                },
            )
        }))
    }

    pub async fn find_user_credentials_by_email(
        deps: &impl GetDb,
        email: &Email,
    ) -> RwResult<Option<(User, Credentials)>> {
        let record = sqlx::query!(
            r#"SELECT user_id, email, username, password_hash, bio, image FROM app.user WHERE email = $1"#,
            email.as_ref()
        )
        .fetch_optional(&deps.get_db().pg_pool)
        .await?;

        Ok(record.map(|record| {
            (
                User {
                    user_id: UserId(record.user_id),
                    username: record.username,
                    bio: record.bio,
                    image: record.image,
                },
                Credentials {
                    email: Email::valid(record.email),
                    password_hash: record.password_hash.into(),
                },
            )
        }))
    }

    pub async fn find_user_by_username(
        deps: &impl GetDb,
        current_user: UserId<Option<uuid::Uuid>>,
        username: &str,
    ) -> RwResult<Option<(User, Following)>> {
        let record = sqlx::query!(
            r#"
            SELECT
                user_id,
                username,
                bio,
                image,
                EXISTS(
                    SELECT 1 FROM app.follow
                    WHERE followed_user_id = "user".user_id AND following_user_id = $2
                ) "following!"
            FROM app.user
            WHERE username = $1
            "#,
            username,
            current_user.0
        )
        .fetch_optional(&deps.get_db().pg_pool)
        .await?;

        Ok(record.map(|record| {
            (
                User {
                    user_id: UserId(record.user_id),
                    username: record.username,
                    bio: record.bio,
                    image: record.image,
                },
                Following(record.following),
            )
        }))
    }

    pub async fn update_user(
        deps: &impl GetDb,
        current_user_id: UserId,
        update: UserUpdate<'_>,
    ) -> RwResult<(User, Credentials)> {
        let record = sqlx::query!(
            // language=PostgreSQL
            r#"
            UPDATE app.user SET
                email = COALESCE($1, email),
                username = COALESCE($2, username),
                password_hash = COALESCE($3, password_hash),
                bio = COALESCE($4, bio),
                image = COALESCE($5, image)
            WHERE user_id = $6
            RETURNING username, bio, image, email, password_hash
            "#,
            update.email,
            update.username,
            update.password_hash.map(|hash| hash.0),
            update.bio,
            update.image,
            current_user_id.0
        )
        .fetch_one(&deps.get_db().pg_pool)
        .await
        .on_constraint("user_username_key", |_| RwError::UsernameTaken)
        .on_constraint("user_email_key", |_| RwError::EmailTaken)?;

        Ok((
            User {
                user_id: current_user_id,
                username: record.username,
                bio: record.bio,
                image: record.image,
            },
            Credentials {
                email: Email::valid(record.email),
                password_hash: record.password_hash.into(),
            },
        ))
    }

    pub async fn insert_follow(
        deps: &impl GetDb,
        current_user_id: UserId,
        username: &str,
    ) -> RwResult<()> {
        let result = sqlx::query!(
            r#"
            WITH id_pair AS (
                SELECT
                    $1::uuid AS following,
                    user_id as followed
                FROM app.user
                WHERE username = $2
            ), insertion AS (
                INSERT INTO app.follow (following_user_id, followed_user_id)
                    SELECT following, followed FROM id_pair
                ON CONFLICT DO NOTHING
                RETURNING 1
            )
            SELECT
                EXISTS(SELECT 1 FROM id_pair) "user_exists!",
                EXISTS(SELECT 1 FROM insertion) "inserted!"
            "#,
            current_user_id.0,
            username
        )
        .fetch_one(&deps.get_db().pg_pool)
        .await
        .on_constraint("follow_following_user_id", |_| RwError::ProfileNotFound)
        .on_constraint("user_cannot_follow_self", |_| RwError::Forbidden)?;

        if !result.user_exists {
            Err(RwError::ProfileNotFound)
        } else {
            Ok(())
        }
    }

    pub async fn delete_follow(
        deps: &impl GetDb,
        current_user_id: UserId,
        username: &str,
    ) -> RwResult<()> {
        let result = sqlx::query!(
            r#"
            WITH other_user AS (
                SELECT user_id FROM app.user WHERE username = $2
            ),
            deleted_follow AS (
                DELETE from app.follow
                WHERE following_user_id = $1
                AND followed_user_id = (SELECT user_id FROM other_user)
                RETURNING 1
            )
            SELECT
                -- This will be `true` if the article existed before we deleted it.
                EXISTS(SELECT 1 FROM other_user) "existed!",
                -- This will only be `true` if we actually deleted the article.
                EXISTS(SELECT 1 FROM deleted_follow) "deleted!"
            "#,
            current_user_id.0,
            username
        )
        .fetch_one(&deps.get_db().pg_pool)
        .await?;

        if !result.existed {
            Err(RwError::ProfileNotFound)
        } else {
            // Note: There is no error code for unfollowing where there was no following in the first place
            Ok(())
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::create_test_db;

    use assert_matches::*;

    pub struct TestNewUser {
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

    pub fn other_user() -> TestNewUser {
        TestNewUser {
            username: "username2",
            email: "email2",
            password_hash: "hash2",
        }
    }

    #[entrait(pub InsertTestUser, unimock = false)]
    pub async fn insert_test_user(
        db: &impl realworld_domain::user::repo::UserRepo,
        user: TestNewUser,
    ) -> RwResult<(User, Credentials)> {
        db.insert_user(
            user.username,
            &user.email.parse().unwrap(),
            user.password_hash.into(),
        )
        .await
    }

    #[tokio::test]
    async fn should_insert_then_fetch_user() -> RwResult<()> {
        let db = create_test_db().await;
        let (created_user, credentials) = db.insert_test_user(TestNewUser::default()).await?;

        assert_eq!("username", created_user.username);
        assert_eq!("email", credentials.email.as_ref());

        let (fetched_user, fetched_credentials) = db
            .find_user_credentials_by_id(created_user.user_id)
            .await?
            .unwrap();
        assert_eq!(created_user, fetched_user);
        assert_eq!(credentials, fetched_credentials);
        Ok(())
    }

    #[tokio::test]
    async fn should_fail_to_create_two_users_with_the_same_username() -> RwResult<()> {
        let db = create_test_db().await;
        db.insert_test_user(TestNewUser::default()).await?;

        let error = db
            .insert_test_user(TestNewUser::default())
            .await
            .expect_err("should error");

        assert_matches!(error, RwError::UsernameTaken);
        Ok(())
    }

    #[tokio::test]
    async fn should_fail_to_create_two_users_with_the_same_email() -> RwResult<()> {
        let db = create_test_db().await;
        db.insert_test_user(TestNewUser::default()).await?;

        let error = db
            .insert_test_user(TestNewUser {
                username: "newusername",
                ..TestNewUser::default()
            })
            .await
            .expect_err("should error");

        assert_matches!(error, RwError::EmailTaken);
        Ok(())
    }

    #[tokio::test]
    async fn should_update_user() -> RwResult<()> {
        let db = create_test_db().await;
        let (created_user, _) = db.insert_test_user(TestNewUser::default()).await?;

        let (updated_user, updated_credentials) = db
            .update_user(
                created_user.user_id,
                UserUpdate {
                    email: Some("newmail"),
                    username: Some("newname"),
                    password_hash: Some("newhash".into()),
                    bio: Some("newbio"),
                    image: Some("newimage"),
                },
            )
            .await?;

        assert_eq!(created_user.user_id, updated_user.user_id);
        assert_eq!("newname", updated_user.username);
        assert_eq!("newbio", updated_user.bio);
        assert_eq!(Some("newimage"), updated_user.image.as_deref());

        assert_eq!("newmail", updated_credentials.email.as_ref());
        assert_eq!("newhash", updated_credentials.password_hash.0);
        Ok(())
    }

    #[tokio::test]
    async fn should_fail_to_update_user_to_taken_username() -> RwResult<()> {
        let db = create_test_db().await;
        db.insert_test_user(TestNewUser::default()).await?;
        let (user, _) = db.insert_test_user(other_user()).await?;

        let error = db
            .update_user(
                user.user_id,
                UserUpdate {
                    username: Some("username"),
                    ..UserUpdate::default()
                },
            )
            .await
            .expect_err("should error");

        assert_matches!(error, RwError::UsernameTaken);
        Ok(())
    }

    #[tokio::test]
    async fn should_fail_to_update_user_to_taken_email() -> RwResult<()> {
        let db = create_test_db().await;
        db.insert_test_user(TestNewUser::default()).await?;
        let (user, _) = db.insert_test_user(other_user()).await?;

        let error = db
            .update_user(
                user.user_id,
                UserUpdate {
                    email: Some("email"),
                    ..UserUpdate::default()
                },
            )
            .await
            .expect_err("should error");

        assert_matches!(error, RwError::EmailTaken);
        Ok(())
    }

    #[tokio::test]
    async fn following_and_unfollowing_should_work() -> RwResult<()> {
        let db = create_test_db().await;
        let (user1, _) = db.insert_test_user(TestNewUser::default()).await?;
        let (user2, _) = db.insert_test_user(other_user()).await?;

        db.insert_follow(user1.user_id, &user2.username).await?;

        assert_matches!(
            db.find_user_by_username(user1.user_id.some(), &user2.username)
                .await?
                .unwrap(),
            (_, Following(true))
        );

        // Idempotent
        db.insert_follow(user1.user_id, &user2.username).await?;

        assert_matches!(
            db.insert_follow(user1.user_id, "unknown")
                .await
                .unwrap_err(),
            RwError::ProfileNotFound
        );

        assert_matches!(
            db.delete_follow(user1.user_id, "unknown")
                .await
                .unwrap_err(),
            RwError::ProfileNotFound
        );

        db.delete_follow(user1.user_id, &user2.username).await?;
        db.delete_follow(user1.user_id, &user2.username).await?;

        assert_matches!(
            db.find_user_by_username(user1.user_id.some(), &user2.username)
                .await?
                .unwrap(),
            (_, Following(false))
        );
        Ok(())
    }

    #[tokio::test]
    async fn follow_unfollow_user_should_fail_on_invalid_current_user() -> RwResult<()> {
        let db = create_test_db().await;
        let (other_user, _) = db.insert_test_user(TestNewUser::default()).await?;
        let err = db
            .insert_follow(UserId(uuid::Uuid::new_v4()), &other_user.username)
            .await
            .unwrap_err();

        assert_matches!(err, RwError::Sqlx(_));
        Ok(())
    }
}
