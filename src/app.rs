use crate::Config;

use entrait::entrait;
use std::sync::Arc;
use time::OffsetDateTime;

#[derive(Clone)]
pub struct App {
    pub config: Arc<Config>,
}

#[entrait(GetJwtSigningKey for App, unimock=test)]
fn get_jwt_signing_key(app: &App) -> &hmac::Hmac<sha2::Sha384> {
    &app.config.jwt_signing_key
}

#[entrait(GetCurrentTime for App, unimock=test)]
fn get_current_time(_: &App) -> OffsetDateTime {
    OffsetDateTime::now_utc()
}
