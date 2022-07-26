use crate::DbResultExt;
use crate::GetDb;

use realworld_core::error::*;
use realworld_core::timestamp::Timestamptz;
use realworld_core::UserId;

use entrait::entrait_export as entrait;
use futures::TryStreamExt;

#[cfg_attr(test, derive(Eq, PartialEq, Debug))]
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

#[derive(Default)]
pub struct Filter<'a> {
    pub slug: Option<&'a str>,
    pub tag: Option<&'a str>,
    pub author: Option<&'a str>,
    pub favorited: Option<&'a str>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[entrait(pub SelectArticles)]
async fn select_articles(
    deps: &impl GetDb,
    user: Option<UserId>,
    filter: Filter<'_>,
) -> RwResult<Vec<Article>> {
    let articles: Vec<Article> = sqlx::query_as!(
        Article,
        // language=PostgreSQL
        r#"
            SELECT
                slug,
                title,
                description,
                body,
                tag_list,
                article.created_at "created_at: Timestamptz",
                article.updated_at "updated_at: Timestamptz",
                EXISTS(
                    SELECT 1 FROM app.article_favorite WHERE user_id = $1
                ) "favorited!",
                coalesce(
                    (SELECT count(*) FROM app.article_favorite fav WHERE fav.article_id = article.article_id),
                    0
                ) "favorites_count!",
                author.username author_username,
                author.bio author_bio,
                author.image author_image,
                EXISTS(
                    SELECT 1 FROM app.follow WHERE followed_user_id = author.user_id AND following_user_id = $1
                ) "following_author!"
            FROM app.article
            INNER JOIN app.user author USING (user_id)
            WHERE (
                $2::text IS NULL OR slug = $2
            ) AND (
                $3::text IS NULL OR tag_list @> array[$3]
            ) AND (
                $4::text IS NULL OR author.username = $4
            ) AND (
                $5::text IS NULL OR EXISTS(
                    SELECT 1
                    FROM app.user
                    INNER JOIN app.article_favorite af USING (user_id)
                    WHERE username = $5
                )
            )
            ORDER BY article.created_at DESC
            LIMIT $6
            OFFSET $7
        "#,
        user.map(|user| user.0),
        filter.slug,
        filter.tag,
        filter.author,
        filter.favorited,
        filter.limit.unwrap_or(20),
        filter.offset.unwrap_or(0)
    )
        .fetch(&deps.get_db().pg_pool)
        .try_collect::<Vec<_>>()
        .await?;

    Ok(articles)
}

#[entrait(pub InsertArticle)]
async fn insert_article(
    deps: &impl GetDb,
    UserId(user_id): UserId,
    slug: &str,
    title: &str,
    description: &str,
    body: &str,
    tag_list: &[String],
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
        tag_list
    )
    .fetch_one(&deps.get_db().pg_pool)
    .await
    .on_constraint("article_slug_key", |_| {
        RwError::DuplicateArticleSlug(slug.to_string())
    })?;

    Ok(article)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create_test_db;
    use crate::user_db::tests as user_db_test;
    use user_db_test::InsertTestUser;

    use realworld_core::iter_util::Single;

    #[entrait(SelectSingle, unimock = false)]
    async fn select_single(db: &impl SelectArticles, filter: Filter<'_>) -> Article {
        db.select_articles(None, filter)
            .await
            .unwrap()
            .into_iter()
            .single()
            .unwrap()
    }

    #[entrait(SelectSingleWithUser, unimock = false)]
    async fn select_single_with_user(
        db: &impl SelectArticles,
        user: Option<UserId>,
        filter: Filter<'_>,
    ) -> Article {
        db.select_articles(user, filter)
            .await
            .unwrap()
            .into_iter()
            .single()
            .unwrap()
    }

    #[tokio::test]
    async fn should_insert_and_fetch_and_list_article() {
        let db = create_test_db().await;
        let user = db.insert_test_user(Default::default()).await.unwrap();

        let inserted_article = insert_article(
            &db,
            UserId(user.id),
            "slug",
            "title",
            "desc",
            "body",
            &["tag".to_string()],
        )
        .await
        .unwrap();

        let fetched_article = db
            .select_single_with_user(
                Some(UserId(user.id)),
                Filter {
                    slug: Some("slug"),
                    ..Default::default()
                },
            )
            .await;
        assert_eq!(fetched_article, inserted_article);

        assert_eq!(inserted_article.slug, "slug");
        assert_eq!(inserted_article.title, "title");
        assert_eq!(inserted_article.description, "desc");
        assert_eq!(inserted_article.body, "body");
        assert_eq!(inserted_article.tag_list, &["tag".to_string()]);

        assert_eq!(inserted_article.created_at.0, inserted_article.updated_at.0);

        assert_eq!(inserted_article.favorited, false);
        assert_eq!(inserted_article.favorites_count, 0);

        assert_eq!(inserted_article.author_username, user.username);
        assert_eq!(inserted_article.author_bio, user.bio);
        assert_eq!(inserted_article.author_image, user.image);
        assert_eq!(inserted_article.following_author, false);
    }

    #[tokio::test]
    async fn should_filter_articles() {
        let db = create_test_db().await;
        let user1 = db.insert_test_user(Default::default()).await.unwrap();
        let user2 = db
            .insert_test_user(user_db_test::other_user())
            .await
            .unwrap();

        db.insert_article(
            UserId(user1.id),
            "slug1",
            "title1",
            "desc1",
            "body1",
            &["tag1".to_string()],
        )
        .await
        .unwrap();

        db.insert_article(
            UserId(user2.id),
            "slug2",
            "title2",
            "desc2",
            "body2",
            &["tag2".to_string()],
        )
        .await
        .unwrap();

        assert_eq!(
            db.select_single(Filter {
                slug: Some("slug1"),
                ..Default::default()
            })
            .await
            .slug,
            "slug1"
        );

        assert_eq!(
            db.select_single(Filter {
                tag: Some("tag1"),
                ..Default::default()
            })
            .await
            .slug,
            "slug1"
        );

        assert_eq!(
            db.select_single(Filter {
                author: Some(&user1.username),
                ..Default::default()
            })
            .await
            .slug,
            "slug1"
        );

        assert_eq!(
            db.select_articles(
                None,
                Filter {
                    favorited: Some(&user1.username),
                    ..Default::default()
                }
            )
            .await
            .unwrap(),
            &[]
        );

        assert_eq!(
            db.select_articles(
                None,
                Filter {
                    offset: Some(1),
                    ..Default::default()
                }
            )
            .await
            .unwrap()
            .len(),
            1
        );
    }
}
