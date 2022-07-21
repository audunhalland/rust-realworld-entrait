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
