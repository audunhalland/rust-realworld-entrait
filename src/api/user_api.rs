use crate::auth::Authenticated;
use crate::error::AppResult;
use crate::user::{self, FetchUser, UpdateUser, UserId};
use crate::user::{CreateUser, Login};

use axum::extract::Extension;
use axum::routing::{get, post};
use axum::{Json, Router};

#[derive(serde::Serialize, serde::Deserialize)]
struct UserBody<T> {
    user: T,
}

type JsonSignedUser = Json<UserBody<user::SignedUser>>;

pub struct UserApi<D>(std::marker::PhantomData<D>);

impl<A> UserApi<A>
where
    A: CreateUser + Login + FetchUser + UpdateUser + Sized + Clone + Send + Sync + 'static,
{
    pub fn router() -> Router {
        Router::new()
            .route("/api/users", post(Self::create))
            .route("/api/users/login", post(Self::login))
            .route("/api/user", get(Self::current_user).put(Self::update_user))
    }

    async fn create(
        app: Extension<A>,
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
    use crate::test_util::*;
    use crate::user::*;
    use axum::http::StatusCode;
    use tower::ServiceExt;
    use unimock::*;

    #[tokio::test]
    async fn unit_test_create_user() {
        let ctx = mock(Some(
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
        let app = UserApi::<Unimock>::router().layer(Extension(ctx.clone()));

        let response = app
            .oneshot(build_json_post_request(
                "/api/users",
                &UserBody {
                    user: user::NewUser {
                        username: "u".to_string(),
                        email: "e".to_string(),
                        password: "p".to_string(),
                    },
                },
            ))
            .await
            .unwrap();

        let (status, bytes) = fetch_json_body(response).await;
        assert_eq!(StatusCode::OK, status);
        let _: UserBody<user::SignedUser> = serde_json::from_slice(&bytes).unwrap();
    }

    #[tokio::test]
    async fn integration_test_create_user() {
        let ctx = spy(None);
        let app = UserApi::<Unimock>::router().layer(Extension(ctx.clone()));

        let response = app
            .oneshot(build_json_post_request(
                "/api/users",
                &UserBody {
                    user: user::NewUser {
                        username: "u".to_string(),
                        email: "e".to_string(),
                        password: "p".to_string(),
                    },
                },
            ))
            .await
            .unwrap();

        let (status, bytes) = fetch_json_body(response).await;
        assert_eq!(StatusCode::OK, status);
        let _: UserBody<user::SignedUser> = serde_json::from_slice(&bytes).unwrap();
    }
}
