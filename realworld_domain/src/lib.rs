use entrait::entrait_export as entrait;

pub mod article;
pub mod comment;
pub mod error;
pub mod iter_util;
pub mod timestamp;
pub mod user;

///
/// Mockable system abstraction
///
#[entrait(mock_api=SystemMock)]
pub trait System {
    fn get_current_time(&self) -> time::OffsetDateTime;
}

///
/// Mockable config accessor
///
#[entrait(mock_api=GetConfigMock)]
pub trait GetConfig {
    fn get_jwt_signing_key(&self) -> &hmac::Hmac<sha2::Sha384>;
}

pub mod test {
    use super::*;
    use unimock::*;

    pub fn mock_jwt_signing_key() -> impl unimock::Clause {
        use hmac::Mac;

        GetConfigMock::get_jwt_signing_key
            .each_call(matching!())
            .returns(
                hmac::Hmac::<sha2::Sha384>::new_from_slice("foobar".as_bytes())
                    .expect("HMAC-SHA-384 can accept any key length"),
            )
    }

    pub fn mock_current_time() -> impl unimock::Clause {
        SystemMock::get_current_time
            .each_call(matching!())
            .returns(time::OffsetDateTime::from_unix_timestamp(0).unwrap())
    }

    pub fn mock_system_and_config() -> impl unimock::Clause {
        (mock_jwt_signing_key(), mock_current_time())
    }
}
