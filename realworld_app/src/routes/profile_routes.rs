use realworld_core::error::RwResult;
use realworld_user::auth::{Authenticate, Token};

use axum::extract::{Extension, Path};
use axum::routing::{get, post};
use axum::Json;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct ProfileBody {
    profile: realworld_user::profile::Profile,
}

pub struct ProfileRoutes<D>(std::marker::PhantomData<D>);

impl<D> ProfileRoutes<D>
where
    D: realworld_user::FetchProfile
        + realworld_user::Follow
        + Authenticate
        + Sized
        + Clone
        + Send
        + Sync
        + 'static,
{
    pub fn router() -> axum::Router {
        axum::Router::new()
            .route("/profiles/:username", get(Self::get_user_profile))
            .route(
                "/profiles/:username/follow",
                post(Self::follow_user).delete(Self::unfollow_user),
            )
    }

    async fn get_user_profile(
        Extension(deps): Extension<D>,
        token: Option<Token>,
        Path(username): Path<String>,
    ) -> RwResult<Json<ProfileBody>> {
        let opt_current_user = token.map(|token| deps.authenticate(token)).transpose()?;
        Ok(Json(ProfileBody {
            profile: deps
                .fetch_profile(opt_current_user.into(), &username)
                .await?,
        }))
    }

    async fn follow_user(
        Extension(deps): Extension<D>,
        token: Token,
        Path(username): Path<String>,
    ) -> RwResult<Json<ProfileBody>> {
        Ok(Json(ProfileBody {
            profile: deps
                .follow(deps.authenticate(token)?, &username, true)
                .await?,
        }))
    }

    async fn unfollow_user(
        Extension(deps): Extension<D>,
        token: Token,
        Path(username): Path<String>,
    ) -> RwResult<Json<ProfileBody>> {
        Ok(Json(ProfileBody {
            profile: deps
                .follow(deps.authenticate(token)?, &username, false)
                .await?,
        }))
    }
}
