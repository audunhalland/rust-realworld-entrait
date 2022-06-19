pub mod error;

#[derive(Clone, Debug)]
pub struct UserId(pub uuid::Uuid);

#[derive(Clone)]
pub struct PasswordHash(pub String);
