mod db;
mod error;
mod types;
mod user;

use std::sync::Arc;

pub struct Config {
    pub jwt_signing_key: hmac::Hmac<sha2::Sha384>,
}

pub struct App {
    pub config: Arc<Config>,
}
