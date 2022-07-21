use crate::config::Config;

use std::sync::Arc;
use time::OffsetDateTime;

#[derive(Clone)]
pub struct App {
    pub config: Arc<Config>,
    pub db: realworld_db::Db,
}

// Implement the leaf dependency from realworld_db for the App.
// `<Impl<T> as GetDb>::get_db` will delegate in its implementation
// back to the 'native' implementation for `T`.
// So here we make the circle complete:
impl realworld_db::GetDb for App {
    fn get_db(&self) -> &realworld_db::Db {
        &self.db
    }
}

impl realworld_core::System for App {
    fn get_current_time(&self) -> time::OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

impl realworld_core::GetConfig for App {
    fn get_jwt_signing_key(&self) -> &hmac::Hmac<sha2::Sha384> {
        &self.config.jwt_signing_key.0
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use unimock::*;

    pub fn mock_jwt_signing_key() -> unimock::Clause {
        use hmac::Mac;

        realworld_core::GetConfig__get_jwt_signing_key
            .each_call(matching!())
            .returns(
                hmac::Hmac::<sha2::Sha384>::new_from_slice("foobar".as_bytes())
                    .expect("HMAC-SHA-384 can accept any key length"),
            )
            .in_any_order()
    }

    pub fn mock_current_time() -> unimock::Clause {
        realworld_core::System__get_current_time
            .each_call(matching!())
            .returns(OffsetDateTime::from_unix_timestamp(0).unwrap())
            .in_any_order()
    }

    pub fn mock_system_and_config() -> unimock::Clause {
        [mock_jwt_signing_key(), mock_current_time()].into()
    }
}
