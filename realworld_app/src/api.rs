mod user_api;

use crate::app::App;

use implementation::Impl;

pub fn api_router() -> axum::Router {
    // This is the order that the modules were authored in.
    user_api::UserApi::<Impl<App>>::router()
}
