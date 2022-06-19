use crate::Config;
use realworld_db::{Db, DbModule};

use entrait::unimock_test::*;
use implementation::Impl;
use std::sync::Arc;
use time::OffsetDateTime;

#[derive(Clone)]
pub struct App {
    pub config: Arc<Config>,
    pub db: Impl<Db>,
}

// Import an "entrait module"
pub trait GetDb {
    type Target: DbModule + Send + Sync;

    fn get_db(&self) -> &Self::Target;
}

impl GetDb for Impl<App> {
    type Target = Impl<Db>;

    fn get_db(&self) -> &Self::Target {
        &self.db
    }
}

impl GetDb for unimock::Unimock {
    type Target = Self;

    fn get_db(&self) -> &Self {
        self
    }
}

#[entrait(pub GetJwtSigningKey)]
fn get_jwt_signing_key(app: &App) -> &hmac::Hmac<sha2::Sha384> {
    &app.config.jwt_signing_key
}

#[entrait(pub GetCurrentTime)]
fn get_current_time(_: &App) -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

#[cfg(test)]
pub mod test {
    use super::*;
    use unimock::*;

    pub fn mock_jwt_signing_key() -> unimock::Clause {
        use hmac::Mac;

        get_jwt_signing_key::Fn::each_call(matching!())
            .returns(
                hmac::Hmac::<sha2::Sha384>::new_from_slice("foobar".as_bytes())
                    .expect("HMAC-SHA-384 can accept any key length"),
            )
            .in_any_order()
    }

    pub fn mock_current_time() -> unimock::Clause {
        get_current_time::Fn::each_call(matching!())
            .returns(OffsetDateTime::from_unix_timestamp(0).unwrap())
            .in_any_order()
    }

    pub fn mock_app_basics() -> unimock::Clause {
        [mock_jwt_signing_key(), mock_current_time()].into()
    }
}
