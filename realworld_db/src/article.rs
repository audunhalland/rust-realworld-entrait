use crate::DbResultExt;
use crate::GetDb;
use crate::OnConstraint;

use realworld_domain::article::repo::*;
use realworld_domain::error::{RwError, RwResult};
use realworld_domain::timestamp::Timestamptz;
use realworld_domain::user::UserId;

use entrait::*;
use futures::TryStreamExt;
use uuid::Uuid;

pub struct PgArticleRepo;

#[entrait]
impl realworld_domain::article::repo::ArticleRepoImpl for PgArticleRepo {
    pub async fn select_articles(
        deps: &impl GetDb,
        current_user: UserId<Option<Uuid>>,
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
                COALESCE(
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
                    FROM app.article_favorite
                    WHERE
                        user_id = (SELECT user_id FROM app.user WHERE username = $5)
                    AND
                        article_id = article.article_id
                )
            ) AND (
                $6::uuid IS NULL OR EXISTS(
                    SELECT 1
                    FROM app.follow
                    WHERE
                        following_user_id = $6
                    AND
                        followed_user_id = author.user_id
                )
            )
            ORDER BY article.created_at DESC
            LIMIT $7
            OFFSET $8
            "#,
            current_user.0,
            filter.slug,
            filter.tag,
            filter.author,
            filter.favorited_by,
            filter.followed_by.map(UserId::into_id),
            filter.limit.unwrap_or(20),
            filter.offset.unwrap_or(0)
        )
        .fetch(&deps.get_db().pg_pool)
        .try_collect::<Vec<_>>()
        .await
        .to_rw_err()?;

        Ok(articles)
    }

    pub async fn fetch_article_id(deps: &impl GetDb, slug: &str) -> RwResult<Uuid> {
        sqlx::query_scalar!(
            // language=PostgreSQL
            "SELECT article_id FROM app.article WHERE slug = $1",
            slug,
        )
        .fetch_optional(&deps.get_db().pg_pool)
        .await
        .to_rw_err()?
        .ok_or(RwError::ArticleNotFound)
    }

    pub async fn insert_article(
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
        .to_rw_err()
        .on_constraint("article_slug_key", |_| {
            RwError::DuplicateArticleSlug(slug.to_string())
        })?;

        Ok(article)
    }

    pub async fn update_article(
        deps: &impl GetDb,
        UserId(user_id): UserId,
        slug: &str,
        up: ArticleUpdate<'_>,
    ) -> RwResult<()> {
        let mut tx = deps.get_db().pg_pool.begin().await.to_rw_err()?;

        let article_meta = sqlx::query!(
            // This locks the `article` row for the duration of the transaction so we're
            // not interleaving this with other possible updates.
            "SELECT article_id, user_id FROM app.article WHERE slug = $1 FOR UPDATE",
            slug
        )
        .fetch_optional(&mut *tx)
        .await
        .to_rw_err()?
        .ok_or(RwError::ArticleNotFound)?;

        if article_meta.user_id != user_id {
            return Err(RwError::Forbidden);
        }

        sqlx::query!(
            // language=PostgreSQL
            r#"
            UPDATE app.article
            SET
                slug = COALESCE($1, slug),
                title = COALESCE($2, title),
                description = COALESCE($3, description),
                body = COALESCE($4, body)
            WHERE article_id = $5
            "#,
            up.slug,
            up.title,
            up.description,
            up.body,
            article_meta.article_id
        )
        .execute(&mut *tx)
        .await
        .to_rw_err()?;

        // Mustn't forget this!
        tx.commit().await.to_rw_err()?;

        Ok(())
    }

    pub async fn delete_article(
        deps: &impl GetDb,
        UserId(user_id): UserId,
        slug: &str,
    ) -> RwResult<()> {
        let result = sqlx::query!(
            // I like to use raw strings for most queries mainly because CLion doesn't try
            // to escape newlines.
            // language=PostgreSQL
            r#"
            WITH deleted_article AS (
                DELETE from app.article
                WHERE slug = $1 AND user_id = $2
                RETURNING 1
            )
            SELECT
                -- This will be `true` if the article existed before we deleted it.
                EXISTS(SELECT 1 FROM app.article WHERE slug = $1) "existed!",
                -- This will only be `true` if we actually deleted the article.
                EXISTS(SELECT 1 FROM deleted_article) "deleted!"
            "#,
            slug,
            user_id
        )
        .fetch_one(&deps.get_db().pg_pool)
        .await
        .to_rw_err()?;

        if result.deleted {
            Ok(())
        } else if result.existed {
            Err(RwError::Forbidden)
        } else {
            Err(RwError::ArticleNotFound)
        }
    }

    pub async fn insert_favorite(
        deps: &impl GetDb,
        UserId(user_id): UserId,
        slug: &str,
    ) -> RwResult<()> {
        sqlx::query_scalar!(
            r#"
            WITH selected_article AS (
                SELECT article_id FROM app.article WHERE slug = $1
            ),
            inserterted_favorite AS (
                INSERT INTO app.article_favorite(article_id, user_id)
                    SELECT article_id, $2 FROM selected_article
                -- if the article is already favorited
                ON CONFLICT DO NOTHING
            )
            SELECT article_id FROM selected_article
            "#,
            slug,
            user_id
        )
        .fetch_optional(&deps.get_db().pg_pool)
        .await
        .to_rw_err()?
        .ok_or(RwError::ArticleNotFound)?;

        Ok(())
    }

    pub async fn delete_favorite(
        deps: &impl GetDb,
        UserId(user_id): UserId,
        slug: &str,
    ) -> RwResult<()> {
        sqlx::query_scalar!(
            r#"
            WITH selected_article AS (
                SELECT article_id FROM app.article WHERE slug = $1
            ),
            deleted_favorite AS (
                DELETE FROM app.article_favorite
                WHERE article_id = (SELECT article_id from selected_article)
                AND user_id = $2
            )
            SELECT article_id FROM selected_article
            "#,
            slug,
            user_id
        )
        .fetch_optional(&deps.get_db().pg_pool)
        .await
        .to_rw_err()?
        .ok_or(RwError::ArticleNotFound)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create_test_db;
    use crate::user::tests as user_db_test;
    use user_db_test::InsertTestUser;

    use realworld_domain::iter_util::Single;

    use assert_matches::*;

    #[entrait(SelectSingleWithUser, unimock = false)]
    async fn select_single_with_user(
        db: &impl ArticleRepo,
        current_user: UserId<Option<Uuid>>,
        filter: Filter<'_>,
    ) -> Article {
        db.select_articles(current_user, filter)
            .await
            .unwrap()
            .into_iter()
            .single()
            .unwrap()
    }

    #[entrait(SelectSingleSlugOrNone, unimock = false)]
    async fn select_single_slug_or_none(
        db: &impl ArticleRepo,
        filter: Filter<'_>,
    ) -> Option<String> {
        db.select_articles(UserId(None), filter)
            .await
            .unwrap()
            .into_iter()
            .single_or_none()
            .unwrap()
            .map(|article| article.slug)
    }

    #[tokio::test]
    async fn article_lifecycle_should_work() -> RwResult<()> {
        let db = create_test_db().await;
        let (user, _) = db.insert_test_user(Default::default()).await?;

        let inserted_article = db
            .insert_article(
                user.user_id,
                "slug",
                "title",
                "desc",
                "body",
                &["tag".to_string()],
            )
            .await?;

        let fetched_article = db
            .select_single_with_user(
                user.user_id.some(),
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

        db.update_article(
            user.user_id,
            "slug",
            ArticleUpdate {
                slug: Some("slug2"),
                title: Some("title2"),
                description: Some("desc2"),
                body: Some("body2"),
            },
        )
        .await?;

        let modified_article = db
            .select_single_with_user(
                user.user_id.some(),
                Filter {
                    slug: Some("slug2"),
                    ..Default::default()
                },
            )
            .await;

        assert_eq!(modified_article.slug, "slug2");
        assert_eq!(modified_article.title, "title2");
        assert_eq!(modified_article.description, "desc2");
        assert_eq!(modified_article.body, "body2");

        db.delete_article(user.user_id, "slug2").await?;

        assert!(db
            .select_articles(
                UserId(None),
                Filter {
                    slug: Some("slug2"),
                    ..Default::default()
                }
            )
            .await?
            .is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn should_filter_articles() -> RwResult<()> {
        let db = create_test_db().await;
        let (user1, _) = db.insert_test_user(Default::default()).await?;
        let (user2, _) = db.insert_test_user(user_db_test::other_user()).await?;

        db.insert_article(
            user1.user_id,
            "slug1",
            "title1",
            "desc1",
            "body1",
            &["tag1".to_string()],
        )
        .await?;

        db.insert_article(
            user2.user_id,
            "slug2",
            "title2",
            "desc2",
            "body2",
            &["tag2".to_string()],
        )
        .await?;

        assert_eq!(
            Some("slug1"),
            db.select_single_slug_or_none(Filter {
                slug: Some("slug1"),
                ..Default::default()
            })
            .await
            .as_deref()
        );

        assert_eq!(
            Some("slug1"),
            db.select_single_slug_or_none(Filter {
                tag: Some("tag1"),
                ..Default::default()
            })
            .await
            .as_deref()
        );

        assert_eq!(
            Some("slug1"),
            db.select_single_slug_or_none(Filter {
                author: Some(&user1.username),
                ..Default::default()
            })
            .await
            .as_deref(),
        );

        assert_eq!(
            None,
            db.select_single_slug_or_none(Filter {
                favorited_by: Some(&user1.username),
                ..Default::default()
            })
            .await
            .as_deref(),
        );

        db.insert_favorite(user1.user_id, "slug1").await?;

        assert_eq!(
            Some("slug1"),
            db.select_single_slug_or_none(Filter {
                favorited_by: Some(&user1.username),
                ..Default::default()
            })
            .await
            .as_deref()
        );

        assert_eq!(
            None,
            db.select_single_slug_or_none(Filter {
                followed_by: Some(user1.user_id),
                ..Default::default()
            })
            .await
            .as_deref()
        );

        assert_eq!(
            db.select_articles(
                UserId(None),
                Filter {
                    offset: Some(1),
                    ..Default::default()
                }
            )
            .await?
            .len(),
            1
        );

        Ok(())
    }

    #[tokio::test]
    async fn updating_article_with_wrong_owner_should_yield_forbidden() -> RwResult<()> {
        let db = create_test_db().await;
        let (user, _) = db.insert_test_user(Default::default()).await?;

        db.insert_article(
            user.user_id,
            "slug",
            "title",
            "desc",
            "body",
            &["tag".to_string()],
        )
        .await?;

        let error = db
            .update_article(UserId(Uuid::new_v4()), "slug", Default::default())
            .await
            .expect_err("Should error");
        assert_matches!(error, RwError::Forbidden);

        Ok(())
    }
}
