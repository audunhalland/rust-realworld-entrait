use entrait::entrait_export as entrait;

use super::password::PasswordHash;
use super::{Email, UserId};
use crate::error::RwResult;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct User {
    pub user_id: UserId,
    pub username: String,
    pub bio: String,
    pub image: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Credentials {
    pub email: Email,
    pub password_hash: PasswordHash,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Following(pub bool);

#[derive(Clone, Default)]
pub struct UserUpdate<'a> {
    pub email: Option<&'a str>,
    pub username: Option<&'a str>,
    pub password_hash: Option<PasswordHash>,
    pub bio: Option<&'a str>,
    pub image: Option<&'a str>,
}

#[entrait(UserRepoImpl, delegate_by=DelegateUserRepo, mock_api=UserRepoMock)]
pub trait UserRepo {
    async fn insert_user(
        &self,
        username: &str,
        email: &Email,
        password_hash: PasswordHash,
    ) -> RwResult<(User, Credentials)>;

    async fn find_user_credentials_by_id(
        &self,
        user_id: UserId,
    ) -> RwResult<Option<(User, Credentials)>>;

    async fn find_user_credentials_by_email(
        &self,
        email: &Email,
    ) -> RwResult<Option<(User, Credentials)>>;

    async fn find_user_by_username(
        &self,
        current_user: UserId<Option<uuid::Uuid>>,
        username: &str,
    ) -> RwResult<Option<(User, Following)>>;

    async fn update_user(
        &self,
        current_user_id: UserId,
        update: UserUpdate<'_>,
    ) -> RwResult<(User, Credentials)>;

    async fn insert_follow(&self, current_user_id: UserId, username: &str) -> RwResult<()>;
    async fn delete_follow(&self, current_user_id: UserId, username: &str) -> RwResult<()>;
}
