use axum::extract::Extension;
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::auth::Auth;
use crate::error::AppResult;
use crate::user;
use crate::user::{CreateUser, Login};
use crate::App;

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
    app: Extension<App>,
    Json(body): Json<UserBody<user::LoginUser>>,
) -> AppResult<JsonSignedUser> {
    Ok(Json(UserBody {
        user: app.login(body.user).await?,
    }))
}

async fn current_user(app: Extension<App>, auth: Auth) -> AppResult<JsonSignedUser> {
    panic!()
}

async fn update_user(
    app: Extension<App>,
    auth: Auth,
    Json(body): Json<UserBody<user::UpdateUser>>,
) -> AppResult<JsonSignedUser> {
    panic!()
}
