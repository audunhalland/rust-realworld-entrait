use crate::auth::Authenticated;
use crate::error::AppResult;
use crate::user::{self, UserId};

use axum::extract::Extension;
use axum::routing::{get, post};
use axum::{Json, Router};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct UserBody<T> {
    user: T,
}

type JsonSignedUser = Json<UserBody<user::SignedUser>>;

pub struct UserApi<D>(std::marker::PhantomData<D>);

impl<A> UserApi<A>
where
    A: user::CreateUser
        + user::Login
        + user::FetchUser
        + user::UpdateUser
        + Sized
        + Clone
        + Send
        + Sync
        + 'static,
{
    pub fn router() -> Router {
        Router::new()
            .route("/users", post(Self::create))
            .route("/users/login", post(Self::login))
            .route("/user", get(Self::current_user).put(Self::update_user))
    }

    async fn create(
        Extension(app): Extension<A>,
        Json(body): Json<UserBody<user::NewUser>>,
    ) -> AppResult<JsonSignedUser> {
        Ok(Json(UserBody {
            user: app.create_user(body.user).await?,
        }))
    }

    async fn login(
        Extension(app): Extension<A>,
        Json(body): Json<UserBody<user::LoginUser>>,
    ) -> AppResult<JsonSignedUser> {
        Ok(Json(UserBody {
            user: app.login(body.user).await?,
        }))
    }

    async fn current_user(
        Extension(app): Extension<A>,
        user_id: Authenticated<UserId>,
    ) -> AppResult<JsonSignedUser> {
        Ok(Json(UserBody {
            user: app.fetch_user(user_id).await?,
        }))
    }

    async fn update_user(
        Extension(app): Extension<A>,
        user_id: Authenticated<UserId>,
        Json(body): Json<UserBody<user::UserUpdate>>,
    ) -> AppResult<JsonSignedUser> {
        Ok(Json(UserBody {
            user: app.update_user(user_id, body.user).await?,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::user_db;
    use crate::db::user_db::DbUser;
    use crate::test_util::*;
    use crate::user::*;
    use axum::http::StatusCode;
    use unimock::*;

    #[tokio::test]
    async fn unit_test_create_user() {
        let unimock = mock(Some(
            create_user::Fn::next_call(matching!(_))
                .answers(|_| {
                    Ok(SignedUser {
                        email: "e".to_string(),
                        token: "e".to_string(),
                        username: "e".to_string(),
                        bio: "e".to_string(),
                        image: None,
                    })
                })
                .once()
                .in_order(),
        ));

        let (status, bytes) = request_json(
            UserApi::<Unimock>::router().layer(Extension(unimock.clone())),
            build_json_post_request(
                "/users",
                &UserBody {
                    user: user::NewUser {
                        username: "username".to_string(),
                        email: "email".to_string(),
                        password: "password".to_string(),
                    },
                },
            ),
        )
        .await;

        assert_eq!(StatusCode::OK, status);
        let _: UserBody<user::SignedUser> = serde_json::from_slice(&bytes).unwrap();
    }

    #[tokio::test]
    async fn integration_test_create_user() {
        let unimock = spy([
            user_db::insert_user::Fn::stub(|each| {
                each.call(matching!("username", "email", _)).answers(
                    move |(username, email, _)| {
                        Ok(DbUser {
                            id: uuid::Uuid::parse_str("20a626ba-c7d3-44c7-981a-e880f81c126f")
                                .unwrap(),
                            username,
                            email,
                            bio: "bio".to_string(),
                            image: None,
                        })
                    },
                );
            }),
            crate::app::test::mock_app_basics(),
        ]);

        let (status, bytes) = request_json(
            UserApi::<Unimock>::router().layer(Extension(unimock.clone())),
            build_json_post_request(
                "/users",
                &UserBody {
                    user: user::NewUser {
                        username: "username".to_string(),
                        email: "email".to_string(),
                        password: "password".to_string(),
                    },
                },
            ),
        )
        .await;

        assert_eq!(StatusCode::OK, status);
        let user_body: UserBody<user::SignedUser> = serde_json::from_slice(&bytes).unwrap();

        assert_eq!("email", user_body.user.email);
        assert_eq!(
            "eyJhbGciOiJIUzM4NCJ9.eyJ1c2VyX2lkIjoiMjBhNjI2YmEtYzdkMy00NGM3LTk4MWEtZTg4MGY4MWMxMjZmIiwiZXhwIjoxMjA5NjAwfQ.u91-bnMtsP2kKhex_lOiam3WkdEfegS3-qs-V06yehzl2Z5WUd4hH7yH7tFh4zSt",
            user_body.user.token
        );
        assert_eq!("username", user_body.user.username);
    }
}
