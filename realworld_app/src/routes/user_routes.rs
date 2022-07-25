use realworld_core::error::RwResult;
use realworld_user::auth::Token;

use axum::extract::Extension;
use axum::routing::{get, post};
use axum::Json;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct UserBody<T> {
    user: T,
}

pub struct UserRoutes<A>(std::marker::PhantomData<A>);

impl<A> UserRoutes<A>
where
    A: realworld_user::CreateUser
        + realworld_user::Login
        + realworld_user::FetchCurrentUser
        + realworld_user::UpdateUser
        + realworld_user::auth::Authenticate
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
        Extension(app): Extension<A>,
        Json(body): Json<UserBody<realworld_user::NewUser>>,
    ) -> RwResult<Json<UserBody<realworld_user::SignedUser>>> {
        Ok(Json(UserBody {
            user: app.create_user(body.user).await?,
        }))
    }

    async fn login(
        Extension(app): Extension<A>,
        Json(body): Json<UserBody<realworld_user::LoginUser>>,
    ) -> RwResult<Json<UserBody<realworld_user::SignedUser>>> {
        Ok(Json(UserBody {
            user: app.login(body.user).await?,
        }))
    }

    async fn current_user(
        Extension(app): Extension<A>,
        token: Token,
    ) -> RwResult<Json<UserBody<realworld_user::SignedUser>>> {
        let user_id = app.authenticate(token)?;
        Ok(Json(UserBody {
            user: app.fetch_current_user(user_id).await?,
        }))
    }

    async fn update_user(
        Extension(app): Extension<A>,
        token: Token,
        Json(body): Json<UserBody<realworld_user::UserUpdate>>,
    ) -> RwResult<Json<UserBody<realworld_user::SignedUser>>> {
        let user_id = app.authenticate(token)?;
        Ok(Json(UserBody {
            user: app.update_user(user_id, body.user).await?,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::*;
    use realworld_core::UserId;
    use realworld_db::user_db;
    use realworld_user::auth::Authenticated;
    use realworld_user::*;

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
            create_user::Fn
                .next_call(matching!(_))
                .answers(|_| Ok(test_signed_user()))
                .once()
                .in_order(),
        ));

        let (status, _) = request_json::<UserBody<realworld_user::SignedUser>>(
            test_router(deps.clone()),
            Request::post("/users").with_json_body(UserBody {
                user: realworld_user::NewUser {
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
            user_db::insert_user::Fn.stub(|each| {
                each.call(matching!("username", "email", _))
                    .answers(|(username, email, _)| {
                        Ok(user_db::User {
                            id: test_uuid(),
                            username,
                            email,
                            bio: "bio".to_string(),
                            image: None,
                        })
                    });
            }),
            realworld_core::test::mock_system_and_config(),
        ]);

        let (status, user_body) = request_json::<UserBody<realworld_user::SignedUser>>(
            test_router(deps.clone()),
            Request::post("/users").with_json_body(UserBody {
                user: realworld_user::NewUser {
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
        let deps = mock([
            auth::authenticate::Fn
                .next_call(matching! {
                    (token) if token.token() == "123"
                })
                .answers(|_| Ok(Authenticated(UserId(test_uuid()))))
                .once()
                .in_order(),
            fetch_current_user::Fn
                .next_call(matching! {
                    (Authenticated(UserId(id))) if id == &test_uuid()
                })
                .answers(|_| Ok(test_signed_user()))
                .once()
                .in_order(),
        ]);

        let (status, _) = request_json::<UserBody<realworld_user::SignedUser>>(
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
