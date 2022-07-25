pub mod auth;
pub mod password;

use auth::Authenticated;

use realworld_core::error::{RwError, RwResult};
use realworld_core::UserId;
use realworld_db::user_db;

use entrait::entrait_export as entrait;

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

#[entrait(pub CreateUser)]
async fn create_user(
    deps: &(impl password::HashPassword + user_db::InsertUser + auth::SignUserId),
    new_user: NewUser,
) -> RwResult<SignedUser> {
    let password_hash = deps.hash_password(new_user.password).await?;

    let db_user = deps
        .insert_user(new_user.username, new_user.email, password_hash)
        .await?;

    Ok(sign_db_user(deps, db_user))
}

#[entrait(pub Login)]
async fn login(
    deps: &(impl user_db::FindUserByEmail + password::VerifyPassword + auth::SignUserId),
    login_user: LoginUser,
) -> RwResult<SignedUser> {
    let (db_user, password_hash) = deps
        .find_user_by_email(login_user.email)
        .await?
        .ok_or(RwError::EmailDoesNotExist)?;

    deps.verify_password(login_user.password, password_hash)
        .await?;

    Ok(sign_db_user(deps, db_user))
}

#[entrait(pub FetchCurrentUser)]
async fn fetch_current_user(
    deps: &(impl user_db::FindUserById + auth::SignUserId),
    Authenticated(user_id): Authenticated<UserId>,
) -> RwResult<SignedUser> {
    let (db_user, _) = deps
        .find_user_by_id(user_id)
        .await?
        .ok_or(RwError::CurrentUserDoesNotExist)?;

    Ok(sign_db_user(deps, db_user))
}

#[entrait(pub UpdateUser)]
async fn update_user(
    deps: &(impl password::HashPassword + user_db::UpdateUser + auth::SignUserId),
    Authenticated(user_id): Authenticated<UserId>,
    update: UserUpdate,
) -> RwResult<SignedUser> {
    let password_hash = if let Some(password) = &update.password {
        Some(deps.hash_password(password.clone()).await?)
    } else {
        None
    };

    Ok(sign_db_user(
        deps,
        deps.update_user(
            user_id,
            user_db::UserUpdate {
                username: update.username,
                email: update.email,
                password_hash,
                bio: update.bio,
                image: update.image,
            },
        )
        .await?,
    ))
}

fn sign_db_user(deps: &impl auth::SignUserId, db_user: user_db::User) -> SignedUser {
    SignedUser {
        email: db_user.email,
        token: deps.sign_user_id(UserId(db_user.id)),
        username: db_user.username,
        bio: db_user.bio,
        image: db_user.image,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use realworld_core::PasswordHash;
    use realworld_db::user_db;

    use unimock::*;

    fn test_token() -> String {
        String::from("t3stt0k1")
    }

    fn test_user_id() -> uuid::Uuid {
        uuid::Uuid::parse_str("20a626ba-c7d3-44c7-981a-e880f81c126f").unwrap()
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
            user_db::insert_user::Fn
                .next_call(matching! {
                    (_, _, PasswordHash(hash)) if hash == "h4sh"
                })
                .answers(|(username, email, _)| {
                    Ok(user_db::User {
                        id: test_user_id(),
                        username,
                        email,
                        bio: "".to_string(),
                        image: None,
                    })
                })
                .once()
                .in_order(),
            auth::sign_user_id::Fn
                .next_call(matching!(_))
                .returns(test_token())
                .once()
                .in_order(),
        ]);

        let signed_user = create_user(&deps, new_user).await.unwrap();

        assert_eq!(signed_user.token, test_token());
    }

    #[tokio::test]
    async fn test_login() {
        let login_user = LoginUser {
            email: "name@email.com".to_string(),
            password: "password".to_string(),
        };
        let deps = mock([
            user_db::find_user_by_email::Fn
                .next_call(matching!("name@email.com"))
                .answers(|email| {
                    Ok(Some((
                        user_db::User {
                            id: test_user_id(),
                            username: "Name".into(),
                            email,
                            bio: "".to_string(),
                            image: None,
                        },
                        PasswordHash("h4sh".into()),
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
