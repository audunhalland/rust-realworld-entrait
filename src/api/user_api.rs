use crate::app::App;
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

pub fn router() -> Router {
    // By having each module responsible for setting up its own routing,
    // it makes the root module a lot cleaner.
    Router::new()
        .route("/api/users", post(create_user))
        .route("/api/users/login", post(login))
        .route("/api/user", get(current_user).put(update_user))
}

async fn create_user(
    app: Extension<App>,
    Json(body): Json<UserBody<user::NewUser>>,
) -> AppResult<JsonSignedUser> {
    Ok(Json(UserBody {
        user: app.create_user(body.user).await?,
    }))
}

async fn login(
    Extension(app): Extension<App>,
    Json(body): Json<UserBody<user::LoginUser>>,
) -> AppResult<JsonSignedUser> {
    Ok(Json(UserBody {
        user: app.login(body.user).await?,
    }))
}

async fn current_user(
    Extension(app): Extension<App>,
    user_id: Authenticated<UserId>,
) -> AppResult<JsonSignedUser> {
    Ok(Json(UserBody {
        user: app.fetch_user(user_id).await?,
    }))
}

async fn update_user(
    Extension(app): Extension<App>,
    user_id: Authenticated<UserId>,
    Json(body): Json<UserBody<user::UserUpdate>>,
) -> AppResult<JsonSignedUser> {
    Ok(Json(UserBody {
        user: app.update_user(user_id, body.user).await?,
    }))
}
