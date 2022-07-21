use entrait::entrait;

pub mod error;

#[derive(Clone, Debug)]
pub struct UserId(pub uuid::Uuid);

#[derive(Clone)]
pub struct PasswordHash(pub String);

///
/// Mockable system abstraction
///
#[entrait]
pub trait System {
    fn get_current_time(&self) -> time::OffsetDateTime;
}

///
/// Mockable config accessor
///
#[entrait]
pub trait GetConfig {
    fn get_jwt_signing_key(&self) -> &hmac::Hmac<sha2::Sha384>;
}

pub mod test {
    use super::*;
    use unimock::*;

    pub fn mock_jwt_signing_key() -> unimock::Clause {
        use hmac::Mac;

        GetConfig__get_jwt_signing_key
            .each_call(matching!())
            .returns(
                hmac::Hmac::<sha2::Sha384>::new_from_slice("foobar".as_bytes())
                    .expect("HMAC-SHA-384 can accept any key length"),
            )
            .in_any_order()
    }

    pub fn mock_current_time() -> unimock::Clause {
        System__get_current_time
            .each_call(matching!())
            .returns(time::OffsetDateTime::from_unix_timestamp(0).unwrap())
            .in_any_order()
    }

    pub fn mock_system_and_config() -> unimock::Clause {
        [mock_jwt_signing_key(), mock_current_time()].into()
    }
}
