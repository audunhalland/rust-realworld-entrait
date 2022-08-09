pub mod auth;
pub mod password;
pub mod profile;
pub mod repo;

use auth::{Authenticate, Token};

use crate::error::{RwError, RwResult};
use crate::UserId;

use entrait::entrait_export as entrait;
use uuid::Uuid;

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
    pub email: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub bio: Option<String>,
    pub image: Option<String>,
}

#[entrait(pub Create)]
async fn create(
    deps: &(impl password::HashPassword + repo::UserRepo + auth::SignUserId),
    new_user: NewUser,
) -> RwResult<SignedUser> {
    let password_hash = deps.hash_password(new_user.password).await?;

    let (db_user, credentials) = deps
        .insert_user(&new_user.username, &new_user.email, password_hash)
        .await?;

    Ok(sign(deps, db_user, credentials.email))
}

#[entrait(pub Login)]
async fn login(
    deps: &(impl repo::UserRepo + password::VerifyPassword + auth::SignUserId),
    login_user: LoginUser,
) -> RwResult<SignedUser> {
    let (db_user, credentials) = deps
        .find_user_credentials_by_email(&login_user.email)
        .await?
        .ok_or(RwError::EmailDoesNotExist)?;

    deps.verify_password(login_user.password, credentials.password_hash)
        .await?;

    Ok(sign(deps, db_user, credentials.email))
}

#[entrait(pub FetchCurrent)]
async fn fetch_current(
    deps: &(impl Authenticate + repo::UserRepo + auth::SignUserId),
    token: Token,
) -> RwResult<SignedUser> {
    let current_user_id = deps.authenticate(token)?;
    let (db_user, credentials) = deps
        .find_user_credentials_by_id(current_user_id)
        .await?
        .ok_or(RwError::CurrentUserDoesNotExist)?;

    Ok(sign(deps, db_user, credentials.email))
}

#[entrait(pub Update)]
async fn update(
    deps: &(impl Authenticate + password::HashPassword + repo::UserRepo + auth::SignUserId),
    token: Token,
    user_update: UserUpdate,
) -> RwResult<SignedUser> {
    let current_user_id = deps.authenticate(token)?;
    let password_hash = if let Some(password) = &user_update.password {
        Some(deps.hash_password(password.clone()).await?)
    } else {
        None
    };

    let (user, credentials) = deps
        .update_user(
            current_user_id,
            repo::UserUpdate {
                username: user_update.username.as_deref(),
                email: user_update.email.as_deref(),
                password_hash,
                bio: user_update.bio.as_deref(),
                image: user_update.image.as_deref(),
            },
        )
        .await?;

    Ok(sign(deps, user, credentials.email))
}

fn sign(deps: &impl auth::SignUserId, db_user: repo::User, email: String) -> SignedUser {
    SignedUser {
        email,
        token: deps.sign_user_id(db_user.user_id),
        username: db_user.username,
        bio: db_user.bio,
        image: db_user.image,
    }
}

#[entrait(pub FetchProfile)]
async fn fetch_profile(
    deps: &(impl Authenticate + repo::UserRepo),
    token: Option<Token>,
    username: &str,
) -> RwResult<profile::Profile> {
    let current_user_id = deps.opt_authenticate(token)?;
    fetch_profile_inner(deps, current_user_id, username).await
}

#[entrait(pub Follow)]
async fn follow(
    deps: &(impl Authenticate + repo::UserRepo),
    token: Token,
    username: &str,
    value: bool,
) -> RwResult<profile::Profile> {
    let current_user_id = deps.authenticate(token)?;
    if value {
        deps.insert_follow(current_user_id, username).await?;
    } else {
        deps.delete_follow(current_user_id, username).await?;
    }
    fetch_profile_inner(deps, current_user_id.some(), username).await
}

async fn fetch_profile_inner(
    deps: &impl repo::UserRepo,
    current_user_id: UserId<Option<Uuid>>,
    username: &str,
) -> RwResult<profile::Profile> {
    let (user, following) = deps
        .find_user_by_username(current_user_id, username)
        .await?
        .ok_or(RwError::ProfileNotFound)?;

    Ok(profile::Profile {
        username: user.username,
        bio: user.bio,
        image: user.image,
        following: following.0,
    })
}

#[cfg(test)]
mod tests {
    use super::repo;
    use super::*;
    use crate::PasswordHash;
    use crate::UserId;

    use unimock::*;

    fn test_token() -> String {
        String::from("t3stt0k1")
    }

    fn test_user_id() -> UserId {
        UserId(uuid::Uuid::parse_str("20a626ba-c7d3-44c7-981a-e880f81c126f").unwrap())
    }

    pub fn mock_hash_password() -> unimock::Clause {
        password::hash_password::Fn
            .next_call(matching!(_))
            .answers(|_| Ok(PasswordHash("h4sh".to_string())))
            .once()
            .in_order()
    }

    #[tokio::test]
    async fn test_create_user() {
        let new_user = NewUser {
            username: "Name".to_string(),
            email: "name@email.com".to_string(),
            password: "password".to_string(),
        };
        let deps = mock([
            mock_hash_password(),
            repo::UserRepo__insert_user
                .next_call(matching!(
                    ("Name", "name@email.com", PasswordHash(hash)) if hash == "h4sh"
                ))
                .answers(|(username, email, hash)| {
                    Ok((
                        repo::User {
                            user_id: test_user_id(),
                            username: username.to_string(),
                            bio: "".to_string(),
                            image: None,
                        },
                        repo::Credentials {
                            email: email.to_string(),
                            password_hash: hash,
                        },
                    ))
                })
                .once()
                .in_order(),
            auth::sign_user_id::Fn
                .next_call(matching!(_))
                .returns(test_token())
                .once()
                .in_order(),
        ]);

        let signed_user = create(&deps, new_user).await.unwrap();

        assert_eq!(signed_user.token, test_token());
    }

    #[tokio::test]
    async fn test_login() {
        let login_user = LoginUser {
            email: "name@email.com".to_string(),
            password: "password".to_string(),
        };
        let deps = mock([
            repo::UserRepo__find_user_credentials_by_email
                .next_call(matching!("name@email.com"))
                .answers(|email| {
                    Ok(Some((
                        repo::User {
                            user_id: test_user_id(),
                            username: "Name".into(),
                            bio: "".to_string(),
                            image: None,
                        },
                        repo::Credentials {
                            email: email.to_string(),
                            password_hash: PasswordHash("h4sh".into()),
                        },
                    )))
                })
                .once()
                .in_order(),
            password::verify_password::Fn
                .next_call(matching!(_))
                .answers(|_| Ok(()))
                .once()
                .in_order(),
            auth::sign_user_id::Fn
                .next_call(matching!(_))
                .returns(test_token())
                .once()
                .in_order(),
        ]);

        let signed_user = login(&deps, login_user).await.unwrap();

        assert_eq!(signed_user.token, test_token());
    }
}
