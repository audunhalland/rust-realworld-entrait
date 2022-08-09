use realworld_domain::error::RwResult;
use realworld_domain::user;
use realworld_domain::user::auth::Token;

use axum::extract::Extension;
use axum::routing::{get, post};
use axum::Json;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct UserBody<T> {
    user: T,
}

pub struct UserRoutes<D>(std::marker::PhantomData<D>);

impl<D> UserRoutes<D>
where
    D: user::Create
        + user::Login
        + user::FetchCurrent
        + user::Update
        + Sized
        + Clone
        + Send
        + Sync
        + 'static,
{
    pub fn router() -> axum::Router {
        axum::Router::new()
            .route("/users", post(Self::create))
            .route("/users/login", post(Self::login))
            .route("/user", get(Self::current_user).put(Self::update_user))
    }

    async fn create(
        Extension(deps): Extension<D>,
        Json(body): Json<UserBody<user::NewUser>>,
    ) -> RwResult<Json<UserBody<user::SignedUser>>> {
        Ok(Json(UserBody {
            user: deps.create(body.user).await?,
        }))
    }

    async fn login(
        Extension(deps): Extension<D>,
        Json(body): Json<UserBody<user::LoginUser>>,
    ) -> RwResult<Json<UserBody<user::SignedUser>>> {
        Ok(Json(UserBody {
            user: deps.login(body.user).await?,
        }))
    }

    async fn current_user(
        Extension(deps): Extension<D>,
        token: Token,
    ) -> RwResult<Json<UserBody<user::SignedUser>>> {
        Ok(Json(UserBody {
            user: deps.fetch_current(token).await?,
        }))
    }

    async fn update_user(
        Extension(deps): Extension<D>,
        token: Token,
        Json(body): Json<UserBody<user::UserUpdate>>,
    ) -> RwResult<Json<UserBody<user::SignedUser>>> {
        Ok(Json(UserBody {
            user: deps.update(token, body.user).await?,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::*;
    use realworld_domain::user::repo::*;
    use realworld_domain::user::UserId;
    use user::*;

    use axum::http::{Request, StatusCode};
    use unimock::*;

    fn test_router(deps: Unimock) -> axum::Router {
        UserRoutes::<Unimock>::router().layer(Extension(deps))
    }

    fn test_uuid() -> uuid::Uuid {
        uuid::Uuid::parse_str("20a626ba-c7d3-44c7-981a-e880f81c126f").unwrap()
    }

    fn test_signed_user() -> SignedUser {
        SignedUser {
            email: "e".to_string(),
            token: "e".to_string(),
            username: "e".to_string(),
            bio: "e".to_string(),
            image: None,
        }
    }

    #[tokio::test]
    async fn unit_test_create_user() {
        let deps = mock(Some(
            create::Fn
                .next_call(matching!(_))
                .answers(|_| Ok(test_signed_user()))
                .once()
                .in_order(),
        ));

        let (status, _) = request_json::<UserBody<user::SignedUser>>(
            test_router(deps.clone()),
            Request::post("/users").with_json_body(UserBody {
                user: user::NewUser {
                    username: "username".to_string(),
                    email: "email".to_string(),
                    password: "password".to_string(),
                },
            }),
        )
        .await
        .unwrap();

        assert_eq!(StatusCode::OK, status);
    }

    #[tokio::test]
    async fn integration_test_create_user() {
        let deps = spy([
            UserRepo__insert_user.stub(|each| {
                each.call(matching!("username", "email", _)).answers(
                    |(username, email, password_hash)| {
                        Ok((
                            repo::User {
                                user_id: UserId(test_uuid()),
                                username: username.to_string(),
                                bio: "bio".to_string(),
                                image: None,
                            },
                            repo::Credentials {
                                email: email.to_string(),
                                password_hash,
                            },
                        ))
                    },
                );
            }),
            realworld_domain::test::mock_system_and_config(),
        ]);

        let (status, user_body) = request_json::<UserBody<user::SignedUser>>(
            test_router(deps.clone()),
            Request::post("/users").with_json_body(UserBody {
                user: user::NewUser {
                    username: "username".to_string(),
                    email: "email".to_string(),
                    password: "password".to_string(),
                },
            }),
        )
        .await
        .unwrap();

        assert_eq!(StatusCode::OK, status);
        assert_eq!("email", user_body.user.email);
        assert_eq!(
            "eyJhbGciOiJIUzM4NCJ9.eyJ1c2VyX2lkIjoiMjBhNjI2YmEtYzdkMy00NGM3LTk4MWEtZTg4MGY4MWMxMjZmIiwiZXhwIjoxMjA5NjAwfQ.u91-bnMtsP2kKhex_lOiam3WkdEfegS3-qs-V06yehzl2Z5WUd4hH7yH7tFh4zSt",
            user_body.user.token
        );
        assert_eq!("username", user_body.user.username);
    }

    #[tokio::test]
    async fn protected_endpoint_with_no_token_should_give_401() {
        let deps = mock(None);
        let (status, _) = request(
            test_router(deps.clone()),
            Request::get("/user").empty_body(),
        )
        .await;
        assert_eq!(StatusCode::UNAUTHORIZED, status);
    }

    #[tokio::test]
    async fn current_user_should_work() {
        let deps = mock(Some(
            fetch_current::Fn
                .next_call(matching!(
                    (token) if token.token() == "123"
                ))
                .answers(|_| Ok(test_signed_user()))
                .once()
                .in_order(),
        ));

        let (status, _) = request_json::<UserBody<user::SignedUser>>(
            test_router(deps.clone()),
            Request::get("/user")
                .header("Authorization", "Token 123")
                .empty_body(),
        )
        .await
        .unwrap();

        assert_eq!(StatusCode::OK, status);
    }
}
