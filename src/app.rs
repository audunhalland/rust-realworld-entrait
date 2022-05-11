use crate::Config;

use entrait::unimock_test::*;
use std::sync::Arc;
use time::OffsetDateTime;

#[derive(Clone)]
pub struct App {
    pub config: Arc<Config>,
}

#[entrait(pub GetJwtSigningKey)]
fn get_jwt_signing_key(app: &App) -> &hmac::Hmac<sha2::Sha384> {
    &app.config.jwt_signing_key
}

#[entrait(pub GetCurrentTime)]
fn get_current_time(_: &App) -> OffsetDateTime {
    OffsetDateTime::now_utc()
}
