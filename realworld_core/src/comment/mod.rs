pub mod repo;

use crate::article::repo::ArticleRepo;
use crate::error::RwResult;
use crate::timestamp::Timestamptz;
use crate::user::auth::Authenticate;
use crate::user::auth::Token;
use crate::user::profile::Profile;
use repo::CommentRepo;

use entrait::entrait_export as entrait;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    id: i64,
    created_at: Timestamptz,
    updated_at: Timestamptz,
    body: String,
    author: Profile,
}

impl From<repo::Comment> for Comment {
    fn from(db: repo::Comment) -> Self {
        Self {
            id: db.comment_id,
            created_at: Timestamptz(db.created_at),
            updated_at: Timestamptz(db.updated_at),
            body: db.body,
            author: Profile {
                username: db.author_username,
                bio: db.author_bio,
                image: db.author_image,
                following: db.following_author,
            },
        }
    }
}

#[entrait(pub Api)]
pub mod api {
    use super::*;

    pub async fn list_comments(
        deps: &(impl Authenticate + ArticleRepo + CommentRepo),
        token: Option<Token>,
        slug: &str,
    ) -> RwResult<Vec<Comment>> {
        let current_user_id = deps.opt_authenticate(token)?;
        let article_id = deps.fetch_id(slug).await?;
        Ok(deps
            .list(current_user_id, article_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn add_comment(
        deps: &(impl Authenticate + CommentRepo),
        token: Token,
        slug: &str,
        body: &str,
    ) -> RwResult<Comment> {
        let current_user_id = deps.authenticate(token)?;
        deps.insert(current_user_id, slug, body)
            .await
            .map(Into::into)
    }

    pub async fn delete_comment(
        deps: &(impl Authenticate + CommentRepo),
        token: Token,
        slug: &str,
        comment_id: i64,
    ) -> RwResult<()> {
        let current_user_id = deps.authenticate(token)?;
        deps.delete(current_user_id, slug, comment_id).await
    }
}
