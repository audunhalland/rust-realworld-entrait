use time::OffsetDateTime;

use entrait::entrait_export as entrait;

use crate::error::RwResult;
use crate::UserId;

use uuid::Uuid;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Comment {
    pub comment_id: i64,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub body: String,
    pub author_username: String,
    pub author_bio: String,
    pub author_image: Option<String>,
    pub following_author: bool,
}

#[entrait(CommentRepoImpl, delegate_by = DelegateCommentRepo)]
pub trait CommentRepo {
    async fn list(
        &self,
        current_user: UserId<Option<Uuid>>,
        article_id: uuid::Uuid,
    ) -> RwResult<Vec<Comment>>;

    async fn insert(
        &self,
        current_user: UserId,
        article_slug: &str,
        body: &str,
    ) -> RwResult<Comment>;

    async fn delete(
        &self,
        current_user: UserId,
        article_slug: &str,
        comment_id: i64,
    ) -> RwResult<()>;
}
