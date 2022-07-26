use crate::DbResultExt;
use crate::GetDb;

use realworld_core::error::*;
use realworld_core::timestamp::Timestamptz;
use realworld_core::UserId;

use entrait::entrait_export as entrait;

pub struct Article {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub body: String,
    pub tag_list: Vec<String>,
    pub created_at: Timestamptz,
    pub updated_at: Timestamptz,
    pub favorited: bool,
    pub favorites_count: i64,
    pub author_username: String,
    pub author_bio: String,
    pub author_image: Option<String>,
    // This was originally `author_following` to match other fields but that's kind of confusing.
    // That made it sound like a flag showing if the author is following the current user
    // but the intent is the other way round.
    pub following_author: bool,
}

#[entrait(pub InsertArticle)]
async fn insert_article(
    deps: &impl GetDb,
    UserId(user_id): UserId,
    slug: String,
    title: String,
    description: String,
    body: String,
    tag_list: Vec<String>,
) -> RwResult<Article> {
    let article = sqlx::query_as!(
        Article,
        // language=PostgreSQL
        r#"
            WITH inserted_article AS (
                INSERT INTO app.article (user_id, slug, title, description, body, tag_list)
                VALUES ($1, $2, $3, $4, $5, $6)
                RETURNING
                    slug,
                    title,
                    description,
                    body,
                    tag_list,
                    -- This is how you can override the inferred type of a column.
                    created_at "created_at: Timestamptz",
                    updated_at "updated_at: Timestamptz"
            )
            SELECT
                inserted_article.*,
                false "favorited!",
                0::int8 "favorites_count!",
                username author_username,
                bio author_bio,
                image author_image,
                -- user is forbidden to follow themselves
                false "following_author!"
            FROM inserted_article
            INNER JOIN app.user ON user_id = $1
        "#,
        user_id,
        slug,
        title,
        description,
        body,
        &tag_list[..]
    )
    .fetch_one(&deps.get_db().pg_pool)
    .await
    .on_constraint("article_slug_key", |_| RwError::DuplicateArticleSlug(slug))?;

    Ok(article)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create_test_db;
    use crate::user_db::tests as user_db_test;

    #[tokio::test]
    async fn should_insert_article() {
        let db = create_test_db().await;
        let user = user_db_test::insert_test_user(&db, Default::default())
            .await
            .unwrap();

        let article = insert_article(
            &db,
            UserId(user.id),
            "slug".to_string(),
            "title".to_string(),
            "desc".to_string(),
            "body".to_string(),
            vec!["tag".to_string()],
        )
        .await
        .unwrap();

        assert_eq!(article.slug, "slug");
        assert_eq!(article.title, "title");
        assert_eq!(article.description, "desc");
        assert_eq!(article.body, "body");
        assert_eq!(article.tag_list, &["tag".to_string()]);

        assert_eq!(article.created_at.0, article.updated_at.0);

        assert_eq!(article.favorited, false);
        assert_eq!(article.favorites_count, 0);

        assert_eq!(article.author_username, user.username);
        assert_eq!(article.author_bio, user.bio);
        assert_eq!(article.author_image, user.image);
        assert_eq!(article.following_author, false);
    }
}
