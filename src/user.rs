use crate::app::{App, GetCurrentTime, GetJwtSigningKey};
use crate::auth::Authenticated;
use crate::db::user_db::*;
use crate::error::*;

use anyhow::Context;
use entrait::*;
use jwt::SignWithKey;
use maplit::*;
use time::OffsetDateTime;

const DEFAULT_SESSION_LENGTH: time::Duration = time::Duration::weeks(2);

pub struct UserId(pub uuid::Uuid);

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SignedUser {
    pub email: String,
    pub token: String,
    pub username: String,
    pub bio: String,
    pub image: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LoginUser {
    pub email: String,
    pub password: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct NewUser {
    username: String,
    email: String,
    password: String,
}

#[derive(serde::Deserialize, Default, PartialEq, Eq)]
#[serde(default)]
pub struct UserUpdate {
    email: Option<String>,
    username: Option<String>,
    password: Option<String>,
    bio: Option<String>,
    image: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct AuthUserClaims {
    user_id: uuid::Uuid,
    /// Standard JWT `exp` claim.
    exp: i64,
}

#[entrait(CreateUser for App, async_trait=true)]
pub async fn create_user(
    deps: &(impl InsertUser + HashPassword + GetCurrentTime + GetJwtSigningKey),
    new_user: NewUser,
) -> AppResult<SignedUser> {
    let password_hash = deps.hash_password(new_user.password).await?;

    let db_user = deps
        .insert_user(new_user.username, new_user.email, password_hash)
        .await?;

    Ok(sign_db_user(
        db_user,
        deps.get_current_time(),
        deps.get_jwt_signing_key(),
    ))
}

#[entrait(Login for App, async_trait=true)]
async fn login(
    deps: &(impl FetchUserAndPasswordHashByEmail + VerifyPassword + GetCurrentTime + GetJwtSigningKey),
    login_user: LoginUser,
) -> AppResult<SignedUser> {
    let (db_user, password_hash) = deps
        .fetch_user_and_password_hash_by_email(login_user.email)
        .await?
        .ok_or(Error::UnprocessableEntity {
            errors: hashmap! {
                "email".into() => vec!["does not exist".into()]
            },
        })?;

    deps.verify_password(login_user.password, password_hash)
        .await?;

    Ok(sign_db_user(
        db_user,
        deps.get_current_time(),
        deps.get_jwt_signing_key(),
    ))
}

#[entrait(FetchUser for App, async_trait=true)]
async fn fetch_user(
    deps: &impl FetchUserAndPasswordHashByEmail,
    user_id: Authenticated<UserId>,
) -> Result<SignedUser, Error> {
    todo!()
}

#[entrait(UpdateUser for App, async_trait=true)]
async fn update_user<D>(
    deps: D,
    user_id: Authenticated<UserId>,
    update: UserUpdate,
) -> Result<SignedUser, Error> {
    todo!()
}

fn sign_db_user(
    db_user: DbUser,
    timestamp: OffsetDateTime,
    signing_key: &hmac::Hmac<sha2::Sha384>,
) -> SignedUser {
    let token = AuthUserClaims {
        user_id: db_user.id,
        exp: (timestamp + DEFAULT_SESSION_LENGTH).unix_timestamp(),
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

#[entrait(HashPassword for App, async_trait=true, unimock=test)]
async fn hash_password(_: &App, password: String) -> AppResult<PasswordHash> {
    use argon2::password_hash::SaltString;
    use argon2::Argon2;

    // Argon2 hashing is designed to be computationally intensive,
    // so we need to do this on a blocking thread.
    Ok(
        tokio::task::spawn_blocking(move || -> AppResult<PasswordHash> {
            let salt = SaltString::generate(rand::thread_rng());
            Ok(PasswordHash(
                argon2::PasswordHash::generate(Argon2::default(), password, salt.as_str())
                    .map_err(|e| anyhow::anyhow!("failed to generate password hash: {}", e))?
                    .to_string(),
            ))
        })
        .await
        .context("panic in generating password hash")??,
    )
}

#[entrait(VerifyPassword for App, async_trait=true, unimock=test)]
async fn verify_password(_: &App, password: String, password_hash: PasswordHash) -> AppResult<()> {
    use argon2::{Argon2, PasswordHash};

    Ok(tokio::task::spawn_blocking(move || -> AppResult<()> {
        let hash = PasswordHash::new(&password_hash.0)
            .map_err(|e| anyhow::anyhow!("invalid password hash: {}", e))?;

        hash.verify_password(&[&Argon2::default()], password)
            .map_err(|e| match e {
                argon2::password_hash::Error::Password => Error::Unauthorized,
                _ => anyhow::anyhow!("failed to verify password hash: {}", e).into(),
            })
    })
    .await
    .context("panic in verifying password hash")??)
}

#[cfg(test)]
mod tests {
    use crate::app::{MockGetCurrentTime, MockGetJwtSigningKey};

    use super::*;

    const TOKEN: &'static str = "eyJhbGciOiJIUzM4NCJ9.eyJ1c2VyX2lkIjoiMjBhNjI2YmEtYzdkMy00NGM3LTk4MWEtZTg4MGY4MWMxMjZmIiwiZXhwIjoxMjA5NjAwfQ.u91-bnMtsP2kKhex_lOiam3WkdEfegS3-qs-V06yehzl2Z5WUd4hH7yH7tFh4zSt";

    fn test_user_id() -> uuid::Uuid {
        uuid::Uuid::parse_str("20a626ba-c7d3-44c7-981a-e880f81c126f").unwrap()
    }

    fn test_password() -> String {
        "password".to_string()
    }

    fn test_password_hash() -> PasswordHash {
        PasswordHash("h4sh".to_string())
    }

    fn setup_hash_password_mock(mock: &mut MockHashPassword) {
        mock.expect_hash_password()
            .returning(|_| Ok(test_password_hash()));
    }

    fn setup_hmac_mock(mock: &mut MockGetJwtSigningKey) {
        use hmac::Mac;
        let hmac = hmac::Hmac::<sha2::Sha384>::new_from_slice("foobar".as_bytes())
            .expect("HMAC-SHA-384 can accept any key length");

        mock.expect_get_jwt_signing_key().once().return_const(hmac);
    }

    fn setup_current_time_mock(mock: &mut MockGetCurrentTime) {
        mock.expect_get_current_time()
            .once()
            .return_const(OffsetDateTime::from_unix_timestamp(0));
    }

    #[tokio::test]
    async fn test_create_user() {
        let new_user = NewUser {
            username: "Name".to_string(),
            email: "name@email.com".to_string(),
            password: test_password(),
        };
        let deps = unimock::Unimock::new()
            .mock(|mock: &mut MockInsertUser| {
                mock.expect_insert_user().returning(|username, email, _| {
                    Ok(DbUser {
                        id: test_user_id(),
                        username,
                        email,
                        bio: "".to_string(),
                        image: None,
                    })
                });
            })
            .mock(setup_hash_password_mock)
            .mock(setup_current_time_mock)
            .mock(setup_hmac_mock);

        let signed_user = create_user(&deps, new_user).await.unwrap();

        assert_eq!(signed_user.token, TOKEN);
    }

    #[tokio::test]
    async fn test_login() {
        let login_user = LoginUser {
            email: "name@email.com".to_string(),
            password: test_password(),
        };
        let deps = unimock::Unimock::new()
            .mock(|mock: &mut MockFetchUserAndPasswordHashByEmail| {
                mock.expect_fetch_user_and_password_hash_by_email()
                    .once()
                    .returning(|email| {
                        Ok(Some((
                            DbUser {
                                id: test_user_id(),
                                username: "Name".into(),
                                email,
                                bio: "".to_string(),
                                image: None,
                            },
                            PasswordHash("".into()),
                        )))
                    });
            })
            .mock(|verify_password: &mut MockVerifyPassword| {
                verify_password
                    .expect_verify_password()
                    .once()
                    .returning(|_, _| Ok(()));
            })
            .mock(setup_current_time_mock)
            .mock(setup_hmac_mock);

        let signed_user = login(&deps, login_user).await.unwrap();

        assert_eq!(signed_user.token, TOKEN);
    }
}
