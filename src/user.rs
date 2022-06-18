use crate::auth;
use crate::db::user_db;
use crate::error::*;

use anyhow::Context;
use entrait::unimock_test::*;
use maplit::*;

#[derive(Clone, Debug)]
pub struct UserId(pub uuid::Uuid);

#[derive(serde::Serialize, serde::Deserialize, Debug)]
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
    pub username: String,
    pub email: String,
    pub password: String,
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

#[entrait(pub CreateUser, async_trait = true)]
async fn create_user(
    deps: &(impl HashPassword + user_db::InsertUser + auth::SignUserId),
    new_user: NewUser,
) -> AppResult<SignedUser> {
    let password_hash = deps.hash_password(new_user.password).await?;

    let db_user = deps
        .insert_user(new_user.username, new_user.email, password_hash)
        .await?;

    Ok(sign_db_user(deps, db_user))
}

#[entrait(pub Login, async_trait = true)]
async fn login(
    deps: &(impl user_db::FetchUserAndPasswordHashByEmail + VerifyPassword + auth::SignUserId),
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

    Ok(sign_db_user(deps, db_user))
}

#[entrait(pub FetchUser, async_trait = true)]
async fn fetch_user(
    deps: &impl user_db::FetchUserAndPasswordHashByEmail,
    user_id: auth::Authenticated<UserId>,
) -> Result<SignedUser, Error> {
    todo!()
}

#[entrait(pub UpdateUser, async_trait = true)]
async fn update_user<D>(
    deps: D,
    user_id: auth::Authenticated<UserId>,
    update: UserUpdate,
) -> Result<SignedUser, Error> {
    todo!()
}

fn sign_db_user(deps: &impl auth::SignUserId, db_user: user_db::DbUser) -> SignedUser {
    SignedUser {
        email: db_user.email,
        token: deps.sign_user_id(UserId(db_user.id)),
        username: db_user.username,
        bio: db_user.bio,
        image: db_user.image,
    }
}

#[entrait(pub HashPassword, async_trait=true, unimock=test)]
async fn hash_password<D>(_: &D, password: String) -> AppResult<user_db::PasswordHash> {
    use argon2::password_hash::SaltString;
    use argon2::Argon2;

    // Argon2 hashing is designed to be computationally intensive,
    // so we need to do this on a blocking thread.
    Ok(
        tokio::task::spawn_blocking(move || -> AppResult<user_db::PasswordHash> {
            let salt = SaltString::generate(rand::thread_rng());
            Ok(user_db::PasswordHash(
                argon2::PasswordHash::generate(Argon2::default(), password, salt.as_str())
                    .map_err(|e| anyhow::anyhow!("failed to generate password hash: {}", e))?
                    .to_string(),
            ))
        })
        .await
        .context("panic in generating password hash")??,
    )
}

#[entrait(VerifyPassword, async_trait=true, unimock=test)]
async fn verify_password<D>(
    _: &D,
    password: String,
    password_hash: user_db::PasswordHash,
) -> AppResult<()> {
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
pub mod test {
    use super::*;
    use unimock::*;

    fn test_password_hash() -> user_db::PasswordHash {
        user_db::PasswordHash("h4sh".to_string())
    }

    pub fn mock_hash_password() -> unimock::Clause {
        hash_password::Fn::each_call(matching!(_))
            .answers(|_| Ok(test_password_hash()))
            .in_any_order()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unimock::*;

    fn test_token() -> String {
        String::from("t3stt0k1")
    }

    fn test_user_id() -> uuid::Uuid {
        uuid::Uuid::parse_str("20a626ba-c7d3-44c7-981a-e880f81c126f").unwrap()
    }

    #[tokio::test]
    async fn test_create_user() {
        let new_user = NewUser {
            username: "Name".to_string(),
            email: "name@email.com".to_string(),
            password: "password".to_string(),
        };
        let mock = mock([
            auth::sign_user_id::Fn::each_call(matching!(_))
                .returns(test_token())
                .in_any_order(),
            user_db::insert_user::Fn::each_call(matching!(_, _, _))
                .answers(|(username, email, _)| {
                    Ok(user_db::DbUser {
                        id: test_user_id(),
                        username,
                        email,
                        bio: "".to_string(),
                        image: None,
                    })
                })
                .in_any_order(),
            super::test::mock_hash_password(),
        ]);

        let signed_user = create_user(&mock, new_user).await.unwrap();

        assert_eq!(signed_user.token, test_token());
    }

    #[tokio::test]
    async fn test_login() {
        let login_user = LoginUser {
            email: "name@email.com".to_string(),
            password: "password".to_string(),
        };
        let mock = mock([
            user_db::fetch_user_and_password_hash_by_email::Fn::each_call(matching!(
                "name@email.com"
            ))
            .answers(|email| {
                Ok(Some((
                    user_db::DbUser {
                        id: test_user_id(),
                        username: "Name".into(),
                        email,
                        bio: "".to_string(),
                        image: None,
                    },
                    user_db::PasswordHash("h4sh".into()),
                )))
            })
            .in_any_order(),
            verify_password::Fn::each_call(matching!(_))
                .answers(|_| (Ok(())))
                .in_any_order(),
            auth::sign_user_id::Fn::each_call(matching!(_))
                .returns(test_token())
                .in_any_order(),
        ]);

        let signed_user = login(&mock, login_user).await.unwrap();

        assert_eq!(signed_user.token, test_token());
    }
}
