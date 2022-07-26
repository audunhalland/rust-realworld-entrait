use crate::DbResultExt;
use crate::GetDb;

use realworld_core::error::*;
use realworld_core::timestamp::Timestamptz;
use realworld_core::UserId;

use entrait::entrait_export as entrait;

pub struct Profile {
    pub username: String,
    pub bio: String,
    pub image: Option<String>,
    pub following: bool,
}

pub struct Article {
    slug: String,
    title: String,
    description: String,
    body: String,
    tag_list: Vec<String>,
    created_at: Timestamptz,
    // Note: the Postman collection included with the spec assumes that this is never null.
    // We prefer to leave it unset unless the row has actually be updated.
    updated_at: Timestamptz,
    favorited: bool,
    favorites_count: i64,
    author: Profile,
}

impl From<ArticleFromQuery> for Article {
    fn from(q: ArticleFromQuery) -> Self {
        Self {
            slug: q.slug,
            title: q.title,
            description: q.description,
            body: q.body,
            tag_list: q.tag_list,
            created_at: q.created_at,
            updated_at: q.updated_at,
            favorited: q.favorited,
            favorites_count: q.favorites_count,
            author: Profile {
                username: q.author_username,
                bio: q.author_bio,
                image: q.author_image,
                following: q.following_author,
            },
        }
    }
}

struct ArticleFromQuery {
    slug: String,
    title: String,
    description: String,
    body: String,
    tag_list: Vec<String>,
    created_at: Timestamptz,
    updated_at: Timestamptz,
    favorited: bool,
    favorites_count: i64,
    author_username: String,
    author_bio: String,
    author_image: Option<String>,
    // This was originally `author_following` to match other fields but that's kind of confusing.
    // That made it sound like a flag showing if the author is following the current user
    // but the intent is the other way round.
    following_author: bool,
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
        ArticleFromQuery,
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

    Ok(article.into())
}
