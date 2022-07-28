use crate::DbResultExt;
use crate::GetDb;

use realworld_core::error::*;
use realworld_core::UserId;

use entrait::entrait_export as entrait;
use futures::TryStreamExt;
use time::OffsetDateTime;
use uuid::Uuid;

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

#[entrait(pub List)]
async fn list(
    deps: &impl GetDb,
    current_user: UserId<Option<Uuid>>,
    article_id: Uuid,
) -> RwResult<Vec<Comment>> {
    let comments = sqlx::query_as!(
        Comment,
        r#"
            SELECT
                comment_id,
                comment.created_at,
                comment.updated_at,
                comment.body,
                author.username author_username,
                author.bio author_bio,
                author.image author_image,
                exists(
                    SELECT 1 FROM app.follow WHERE followed_user_id = author.user_id AND following_user_id = $1
                ) "following_author!"
            FROM app.article_comment comment
            INNER JOIN app.user author using (user_id)
            WHERE article_id = $2
            ORDER by created_at
        "#,
        current_user.0,
        article_id
    )
        .fetch(&deps.get_db().pg_pool)
        .try_collect()
        .await?;

    Ok(comments)
}

#[entrait(pub Insert)]
async fn insert(
    deps: &impl GetDb,
    current_user: UserId<Uuid>,
    article_slug: &str,
    body: &str,
) -> RwResult<Comment> {
    let comment = sqlx::query_as!(
        Comment,
        r#"
            WITH inserted_comment AS (
                INSERT INTO app.article_comment (article_id, user_id, body)
                    SELECT article_id, $1, $2
                    FROM app.article
                    WHERE slug = $3
                RETURNING comment_id, created_at, updated_at, body
            )
            SELECT
                comment_id,
                comment.created_at,
                comment.updated_at,
                body,
                author.username author_username,
                author.bio author_bio,
                author.image author_image,
                false "following_author!"
            FROM inserted_comment comment
            INNER JOIN app.user author ON user_id = $1
        "#,
        current_user.0,
        body,
        article_slug,
    )
    .fetch_optional(&deps.get_db().pg_pool)
    .await?
    .ok_or(RwError::ArticleNotFound)?;

    Ok(comment)
}
