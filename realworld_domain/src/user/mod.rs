pub mod auth;
pub mod email;
pub mod password;
pub mod profile;
pub mod repo;

use auth::{Authenticate, Token};
use email::Email;
use password::CleartextPassword;

use crate::error::{RwError, RwResult};

use entrait::entrait_export as entrait;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserId<I = uuid::Uuid>(pub I);

impl<I> UserId<I> {
    pub fn into_id(self) -> I {
        self.0
    }

    pub fn some(self) -> UserId<Option<I>> {
        UserId(Some(self.0))
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct SignedUser {
    pub email: Email,
    pub token: String,
    pub username: String,
    pub bio: String,
    pub image: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LoginUser {
    pub email: Email,
    pub password: CleartextPassword,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub password: CleartextPassword,
}

#[derive(serde::Deserialize, Default, PartialEq, Eq)]
#[serde(default)]
pub struct UserUpdate {
    pub email: Option<String>,
    pub username: Option<String>,
    pub password: Option<CleartextPassword>,
    pub bio: Option<String>,
    pub image: Option<String>,
}

#[entrait(pub Create, mock_api=CreateMock)]
async fn create(
    deps: &(impl password::HashPassword + repo::UserRepo + auth::SignUserId),
    new_user: NewUser,
) -> RwResult<SignedUser> {
    let email = new_user.email.parse()?;
    let password_hash = deps.hash_password(new_user.password).await?;

    let (user, credentials) = deps
        .insert_user(&new_user.username, &email, password_hash)
        .await?;

    Ok(user.sign(deps, credentials.email))
}

#[entrait(pub Login)]
async fn login(
    deps: &(impl repo::UserRepo + password::VerifyPassword + auth::SignUserId),
    login_user: LoginUser,
) -> RwResult<SignedUser> {
    let (user, credentials) = deps
        .find_user_credentials_by_email(&login_user.email)
        .await?
        .ok_or(RwError::EmailDoesNotExist)?;

    deps.verify_password(login_user.password, credentials.password_hash)
        .await?;

    Ok(user.sign(deps, credentials.email))
}

#[entrait(pub FetchCurrent, mock_api=FetchCurrentMock)]
async fn fetch_current(
    deps: &(impl Authenticate + repo::UserRepo + auth::SignUserId),
    token: Token,
) -> RwResult<SignedUser> {
    let current_user_id = deps.authenticate(token)?;
    let (user, credentials) = deps
        .find_user_credentials_by_id(current_user_id)
        .await?
        .ok_or(RwError::CurrentUserDoesNotExist)?;

    Ok(user.sign(deps, credentials.email))
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

    Ok(user.sign(deps, credentials.email))
}

impl repo::User {
    fn sign(self, deps: &impl auth::SignUserId, email: Email) -> SignedUser {
        SignedUser {
            email,
            token: deps.sign_user_id(self.user_id),
            username: self.username,
            bio: self.bio,
            image: self.image,
        }
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
    use super::password::{HashPassword, HashPasswordMock};
    use super::repo;
    use super::*;

    use assert_matches::*;
    use unimock::*;

    fn test_token() -> String {
        String::from("t3stt0k1")
    }

    fn test_user_id() -> UserId {
        UserId(uuid::Uuid::parse_str("20a626ba-c7d3-44c7-981a-e880f81c126f").unwrap())
    }

    fn test_repo_user() -> repo::User {
        repo::User {
            user_id: test_user_id(),
            username: "Name".into(),
            bio: "".to_string(),
            image: None,
        }
    }

    pub fn mock_hash_password() -> impl unimock::Clause {
        HashPasswordMock
            .next_call(matching!(_))
            .returns(Ok("h4sh".into()))
    }

    #[tokio::test]
    async fn test_create_user() {
        let deps = Unimock::new((
            mock_hash_password(),
            repo::UserRepoMock::insert_user
                .next_call(matching!("Name", "name@email.com", "h4sh"))
                .answers(|(username, email, password_hash)| {
                    Ok((
                        repo::User {
                            user_id: test_user_id(),
                            username: username.to_string(),
                            bio: "".to_string(),
                            image: None,
                        },
                        repo::Credentials {
                            email: email.clone(),
                            password_hash,
                        },
                    ))
                }),
            auth::SignUserIdMock
                .next_call(matching!(_))
                .returns(test_token()),
        ));

        let signed_user = create(
            &deps,
            NewUser {
                username: "Name".to_string(),
                email: "name@email.com".parse().unwrap(),
                password: "password".into(),
            },
        )
        .await
        .unwrap();

        assert_eq!(signed_user.token, test_token());
    }

    #[tokio::test]
    async fn test_login_ok() {
        let deps = Unimock::new((
            repo::UserRepoMock::find_user_credentials_by_email
                .next_call(matching!("name@email.com"))
                .answers(|email| {
                    Ok(Some((
                        test_repo_user(),
                        repo::Credentials {
                            email: email.clone(),
                            password_hash: "h4sh".into(),
                        },
                    )))
                }),
            password::VerifyPasswordMock
                .next_call(matching!(_))
                .returns(Ok(())),
            auth::SignUserIdMock
                .next_call(matching!(_))
                .returns(test_token()),
        ));

        let signed_user = login(
            &deps,
            LoginUser {
                email: "name@email.com".parse().unwrap(),
                password: "password".into(),
            },
        )
        .await
        .unwrap();

        assert_eq!(signed_user.token, test_token());
    }

    #[tokio::test]
    async fn integration_test_mismatched_password() {
        let wrong_password_hash = ::entrait::Impl::new(())
            .hash_password("wrong_password".into())
            .await
            .unwrap();

        let deps = Unimock::new_partial(
            repo::UserRepoMock::find_user_credentials_by_email
                .next_call(matching!("name@email.com"))
                .answers(move |email| {
                    Ok(Some((
                        test_repo_user(),
                        repo::Credentials {
                            email: email.clone(),
                            password_hash: wrong_password_hash.clone(),
                        },
                    )))
                }),
        );

        let error = login(
            &deps,
            LoginUser {
                email: "name@email.com".parse().unwrap(),
                password: "password".into(),
            },
        )
        .await
        .expect_err("should error");

        assert_matches!(error, RwError::Unauthorized);
    }
}
