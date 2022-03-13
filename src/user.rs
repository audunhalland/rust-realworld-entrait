use crate::db::user_db::*;
use crate::error::*;

use anyhow::Context;
use entrait::*;
use jwt::SignWithKey;
use time::OffsetDateTime;

const DEFAULT_SESSION_LENGTH: time::Duration = time::Duration::weeks(2);

pub struct SignedUser {
    pub email: String,
    pub token: String,
    pub username: String,
    pub bio: String,
    pub image: Option<String>,
}

pub struct LoginUser {
    pub email: String,
    pub password: String,
}

pub struct NewUser {
    username: String,
    email: String,
    password: String,
}

pub struct AuthUser {
    pub user_id: uuid::Uuid,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct AuthUserClaims {
    user_id: uuid::Uuid,
    /// Standard JWT `exp` claim.
    exp: i64,
}

#[entrait(CreateUser for crate::App, async_trait=true)]
async fn create_user(
    deps: &(impl InsertUser + GetJwtSigningKey),
    new_user: NewUser,
) -> Result<SignedUser> {
    let password_hash = hash_password(new_user.password).await?;

    let db_user = deps
        .insert_user(new_user.username, new_user.email, password_hash)
        .await?;

    Ok(sign_db_user(db_user, deps.get_jwt_signing_key()))
}

#[entrait(Login for crate::App, async_trait=true)]
async fn login(
    deps: &(impl FetchUserByEmail + GetJwtSigningKey),
    login_user: LoginUser,
) -> Result<Option<SignedUser>> {
    let db_user = deps.fetch_user_by_email(login_user.email).await?;

    Ok(db_user.map(|db_user| sign_db_user(db_user, deps.get_jwt_signing_key())))
}

fn sign_db_user(db_user: DbUser, signing_key: &hmac::Hmac<sha2::Sha384>) -> SignedUser {
    let token = AuthUserClaims {
        user_id: db_user.id,
        exp: (OffsetDateTime::now_utc() + DEFAULT_SESSION_LENGTH).unix_timestamp(),
    }
    .sign_with_key(signing_key)
    .expect("HMAC signing should be infallible");

    SignedUser {
        email: db_user.email,
        token,
        username: db_user.username,
        bio: db_user.bio,
        image: db_user.image,
    }
}

#[entrait(GetJwtSigningKey for crate::App, test_unimock=true)]
fn get_jwt_signing_key(app: &crate::App) -> &hmac::Hmac<sha2::Sha384> {
    &app.config.jwt_signing_key
}

async fn hash_password(password: String) -> Result<String> {
    use argon2::password_hash::SaltString;
    use argon2::{Argon2, PasswordHash};

    // Argon2 hashing is designed to be computationally intensive,
    // so we need to do this on a blocking thread.
    Ok(tokio::task::spawn_blocking(move || -> Result<String> {
        let salt = SaltString::generate(rand::thread_rng());
        Ok(
            PasswordHash::generate(Argon2::default(), password, salt.as_str())
                .map_err(|e| anyhow::anyhow!("failed to generate password hash: {}", e))?
                .to_string(),
        )
    })
    .await
    .context("panic in generating password hash")??)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_hmac_mock(mock: &mut MockGetJwtSigningKey) {
        use hmac::NewMac;
        let hmac = hmac::Hmac::<sha2::Sha384>::new_from_slice("foobar".as_bytes())
            .expect("HMAC-SHA-384 can accept any key length");

        mock.expect_get_jwt_signing_key().return_const(hmac);
    }

    #[tokio::test]
    async fn test_create_user() {
        let new_user = NewUser {
            username: "Name".to_string(),
            email: "name@email.com".to_string(),
            password: "password".to_string(),
        };
        let deps = unimock::Unimock::new()
            .mock(|insert_user: &mut MockInsertUser| {
                insert_user
                    .expect_insert_user()
                    .returning(|username, email, _| {
                        Ok(DbUser {
                            id: uuid::Uuid::new_v4(),
                            username,
                            email,
                            bio: "".to_string(),
                            image: None,
                        })
                    });
            })
            .mock(setup_hmac_mock);

        let signed_user = create_user(&deps, new_user).await.unwrap();

        assert_eq!("Name", &signed_user.username);
    }
}
