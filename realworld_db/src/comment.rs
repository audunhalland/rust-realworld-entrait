use crate::GetDb;

use realworld_domain::comment::repo::Comment;
use realworld_domain::error::*;
use realworld_domain::user::UserId;

use futures::TryStreamExt;
use uuid::Uuid;

use entrait::*;

pub struct PgCommentRepo;

#[entrait]
impl realworld_domain::comment::repo::CommentRepoImpl for PgCommentRepo {
    pub async fn list_comments(
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

    pub async fn insert_comment(
        deps: &impl GetDb,
        current_user: UserId,
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

    pub async fn delete_comment(
        deps: &impl GetDb,
        current_user: UserId,
        article_slug: &str,
        comment_id: i64,
    ) -> RwResult<()> {
        let result = sqlx::query!(
            r#"
            WITH deleted_comment AS (
                DELETE FROM app.article_comment
                WHERE
                    comment_id = $1
                AND
                    article_id IN (SELECT article_id FROM app.article WHERE slug = $2)
                AND
                    user_id = $3
                RETURNING 1
            )
            SELECT
                EXISTS(
                    SELECT 1 FROM app.article_comment
                    INNER JOIN app.article USING (article_id)
                    WHERE comment_id = $1 AND slug = $2
                ) "existed!",
                EXISTS(SELECT 1 FROM deleted_comment) "deleted!"
            "#,
            comment_id,
            article_slug,
            current_user.0
        )
        .fetch_one(&deps.get_db().pg_pool)
        .await?;

        if result.deleted {
            Ok(())
        } else if result.existed {
            Err(RwError::Forbidden)
        } else {
            Err(RwError::ArticleNotFound)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create_test_db;
    use crate::user::tests as user_db_test;
    use user_db_test::InsertTestUser;

    use realworld_domain::article::repo::ArticleRepo;
    use realworld_domain::comment::repo::CommentRepo;

    async fn insert_test_article(deps: &impl ArticleRepo, current_user: UserId) -> RwResult<()> {
        deps.insert_article(
            current_user,
            "slug",
            "title",
            "desc",
            "body",
            &["tag".to_string()],
        )
        .await?;
        Ok(())
    }

    #[tokio::test]
    async fn comment_lifecycle() -> RwResult<()> {
        let db = create_test_db().await;
        let (user, _) = db.insert_test_user(Default::default()).await?;
        insert_test_article(&db, user.user_id).await?;
        let article_id = db.fetch_article_id("slug").await?;

        let inserted_comment = db.insert_comment(user.user_id, "slug", "body").await?;

        assert_eq!(
            db.list_comments(user.user_id.some(), article_id).await?,
            &[inserted_comment.clone()]
        );

        assert_eq!(
            db.list_comments(user.user_id.some(), Uuid::new_v4())
                .await?,
            &[]
        );

        db.delete_comment(user.user_id, "slug", inserted_comment.comment_id)
            .await?;

        assert_eq!(
            db.list_comments(user.user_id.some(), article_id).await?,
            &[]
        );

        Ok(())
    }
}
