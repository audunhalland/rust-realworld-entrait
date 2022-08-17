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

impl realworld_domain::System for App {
    fn get_current_time(&self) -> time::OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

impl realworld_domain::GetConfig for App {
    fn get_jwt_signing_key(&self) -> &hmac::Hmac<sha2::Sha384> {
        &self.config.jwt_signing_key.0
    }
}

impl realworld_domain::user::repo::DelegateUserRepo<Self> for App {
    type Target = realworld_db::user::PgUserRepo;
}

impl realworld_domain::article::repo::DelegateArticleRepo<Self> for App {
    type Target = realworld_db::article::PgArticleRepo;
}

impl realworld_domain::comment::repo::DelegateCommentRepo<Self> for App {
    type Target = realworld_db::comment::PgCommentRepo;
}
