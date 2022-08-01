use realworld_core::error::*;
use realworld_core::iter_util::Single;
use realworld_core::timestamp::Timestamptz;
use realworld_core::UserId;
use realworld_db::article_db;
use realworld_db::comment_db;
use realworld_user::auth::*;
use realworld_user::profile::Profile;

use entrait::entrait_export as entrait;

#[derive(serde::Deserialize, serde::Serialize, Clone)]
#[cfg_attr(test, derive(Debug))]
#[serde(rename_all = "camelCase")]
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

impl From<article_db::Article> for Article {
    fn from(q: article_db::Article) -> Self {
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

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    id: i64,
    created_at: Timestamptz,
    updated_at: Timestamptz,
    body: String,
    author: Profile,
}

impl From<comment_db::Comment> for Comment {
    fn from(db: comment_db::Comment) -> Self {
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

#[derive(serde::Deserialize, serde::Serialize)]
// The Realworld spec doesn't mention this as an API convention, it just finally shows up
// when you're looking at the spec for the Article object and see `tagList` as a field name.
#[serde(rename_all = "camelCase")]
pub struct ArticleCreate {
    title: String,
    description: String,
    body: String,
    tag_list: Vec<String>,
}

#[derive(serde::Deserialize)]
pub struct ArticleUpdate {
    title: Option<String>,
    description: Option<String>,
    body: Option<String>,
}

#[derive(serde::Deserialize, Default, Eq, PartialEq)]
#[serde(default)]
pub struct ListArticlesQuery {
    tag: Option<String>,
    author: Option<String>,
    favorited: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(serde::Deserialize, Default)]
#[serde(default)]
pub struct FeedArticlesQuery {
    // See comment on these fields in `ListArticlesQuery` above.
    limit: Option<i64>,
    offset: Option<i64>,
}

#[entrait(pub Api)]
pub mod api {
    use super::*;

    pub async fn list_articles(
        deps: &(impl OptAuthenticate + article_db::Select),
        token: Option<Token>,
        query: ListArticlesQuery,
    ) -> RwResult<Vec<Article>> {
        let current_user_id = deps.opt_authenticate(token)?;
        deps.select(
            current_user_id,
            article_db::Filter {
                slug: None,
                tag: query.tag.as_deref(),
                author: query.author.as_deref(),
                favorited_by: query.favorited.as_deref(),
                followed_by: None,
                limit: query.limit,
                offset: query.offset,
            },
        )
        .await
        .map(|articles| articles.into_iter().map(Into::into).collect())
    }

    pub async fn feed_articles(
        deps: &(impl Authenticate + article_db::Select),
        token: Token,
        query: FeedArticlesQuery,
    ) -> RwResult<Vec<Article>> {
        let current_user_id = deps.authenticate(token)?;
        deps.select(
            current_user_id.some(),
            article_db::Filter {
                slug: None,
                tag: None,
                author: None,
                favorited_by: None,
                followed_by: Some(current_user_id),
                limit: query.limit,
                offset: query.offset,
            },
        )
        .await
        .map(|articles| articles.into_iter().map(Into::into).collect())
    }

    pub async fn fetch_article(
        deps: &(impl OptAuthenticate + article_db::Select),
        token: Option<Token>,
        slug: &str,
    ) -> RwResult<Article> {
        let current_user_id = deps.opt_authenticate(token)?;
        deps.select(
            current_user_id,
            article_db::Filter {
                slug: Some(slug),
                ..Default::default()
            },
        )
        .await?
        .into_iter()
        .single_or_none()?
        .map(Into::into)
        .ok_or(RwError::ArticleNotFound)
    }

    pub async fn create_article(
        deps: &(impl Authenticate + article_db::Insert),
        token: Token,
        article: ArticleCreate,
    ) -> RwResult<Article> {
        let current_user_id = deps.authenticate(token)?;
        let slug = slugify(&article.title);
        deps.insert(
            current_user_id,
            &slug,
            &article.title,
            &article.description,
            &article.body,
            &article.tag_list,
        )
        .await
        .map(Into::into)
    }

    pub async fn update_article(
        deps: &(impl Authenticate + article_db::Update + article_db::Select),
        token: Token,
        slug: &str,
        article_update: ArticleUpdate,
    ) -> RwResult<Article> {
        let current_user_id = deps.authenticate(token)?;
        let new_slug = article_update.title.as_deref().map(slugify);

        deps.update(
            current_user_id,
            slug,
            article_db::ArticleUpdate {
                slug: new_slug.as_deref(),
                title: article_update.title.as_deref(),
                description: article_update.description.as_deref(),
                body: article_update.body.as_deref(),
            },
        )
        .await?;

        get_single_article(deps, current_user_id, new_slug.as_deref().unwrap_or(slug)).await
    }

    pub async fn delete_article(
        deps: &(impl Authenticate + article_db::Delete),
        token: Token,
        slug: &str,
    ) -> RwResult<()> {
        let current_user_id = deps.authenticate(token)?;
        deps.delete(current_user_id, slug).await
    }

    pub async fn favorite_article(
        deps: &(impl Authenticate
              + article_db::InsertFavorite
              + article_db::DeleteFavorite
              + article_db::Select),
        token: Token,
        slug: &str,
        value: bool,
    ) -> RwResult<Article> {
        let current_user_id = deps.authenticate(token)?;
        if value {
            deps.insert_favorite(current_user_id, slug).await?;
        } else {
            deps.delete_favorite(current_user_id, slug).await?;
        }
        get_single_article(deps, current_user_id, slug).await
    }

    pub async fn list_comments(
        deps: &(impl OptAuthenticate + article_db::FetchId + comment_db::List),
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
        deps: &(impl Authenticate + comment_db::Insert),
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
        deps: &(impl Authenticate + comment_db::Delete),
        token: Token,
        slug: &str,
        comment_id: i64,
    ) -> RwResult<()> {
        let current_user_id = deps.authenticate(token)?;
        deps.delete(current_user_id, slug, comment_id).await
    }
}

async fn get_single_article(
    deps: &impl article_db::Select,
    current_user_id: UserId,
    slug: &str,
) -> RwResult<Article> {
    deps.select(
        current_user_id.some(),
        article_db::Filter {
            slug: Some(slug),
            ..Default::default()
        },
    )
    .await?
    .into_iter()
    .single()
    .map(Into::into)
}

fn slugify(string: &str) -> String {
    use itertools::Itertools;

    const QUOTE_CHARS: &[char] = &['\'', '"'];

    string
        // Split on anything that isn't a word character or quotation mark.
        // This has the effect of keeping contractions and possessives together.
        .split(|c: char| !(QUOTE_CHARS.contains(&c) || c.is_alphanumeric()))
        // If multiple non-word characters follow each other then we'll get empty substrings
        // so we'll filter those out.
        .filter(|s| !s.is_empty())
        .map(|s| {
            // Remove quotes from the substring.
            //
            // This allocation is probably avoidable with some more iterator hackery but
            // at that point we'd be micro-optimizing. This function isn't called all that often.
            let mut s = s.replace(QUOTE_CHARS, "");
            // Make the substring lowercase (in-place operation)
            s.make_ascii_lowercase();
            s
        })
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::*;
    use unimock::*;
    use uuid::Uuid;

    fn test_timestamp() -> Timestamptz {
        Timestamptz(
            time::OffsetDateTime::parse(
                "2019-10-12T07:20:50.52Z",
                &time::format_description::well_known::Rfc3339,
            )
            .unwrap(),
        )
    }

    fn test_db_article() -> article_db::Article {
        article_db::Article {
            slug: "slug".to_string(),
            title: "title".to_string(),
            description: "desc".to_string(),
            body: "body".to_string(),
            tag_list: vec!["tag".to_string()],
            created_at: test_timestamp(),
            updated_at: test_timestamp(),
            favorited: false,
            favorites_count: 0,
            author_username: "author".to_string(),
            author_bio: "bio".to_string(),
            author_image: Some("image".to_string()),
            following_author: false,
        }
    }

    fn mock_authenticate() -> unimock::Clause {
        authenticate::Fn
            .next_call(matching!(_))
            .answers(|_| Ok(UserId(Uuid::new_v4())))
            .once()
            .in_order()
    }

    fn mock_authenticate_anonymous() -> unimock::Clause {
        opt_authenticate::Fn
            .next_call(matching!(None))
            .answers(|_| Ok(UserId(None)))
            .once()
            .in_order()
    }

    #[tokio::test]
    async fn create_article_should_slugify() {
        let deps = mock([
            mock_authenticate(),
            article_db::insert::Fn
                .next_call(matching!(UserId(_), "my-title", _, _, _, _))
                .answers(|_| Ok(test_db_article()))
                .once()
                .in_order(),
        ]);
        api::create_article(
            &deps,
            Token::from_token("token"),
            ArticleCreate {
                title: "My Title".to_string(),
                description: "Desc".to_string(),
                body: "Body".to_string(),
                tag_list: vec!["tag".to_string()],
            },
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn get_article_empty_result_should_produce_not_found_error() {
        let deps = mock([
            mock_authenticate_anonymous(),
            article_db::select::Fn
                .next_call(matching!(
                    UserId(None),
                    article_db::Filter {
                        slug: Some("slug"),
                        ..
                    }
                ))
                .answers(|_| Ok(vec![]))
                .once()
                .in_order(),
        ]);
        assert_matches!(
            api::fetch_article(&deps, Token::none(), "slug").await,
            Err(RwError::ArticleNotFound)
        );
    }

    #[tokio::test]
    async fn update_article_should_update_slug() {
        let deps = mock([
            mock_authenticate(),
            article_db::update::Fn
                .next_call(matching!(
                    UserId(_),
                    "slug",
                    article_db::ArticleUpdate {
                        slug: Some("new-title"),
                        title: Some("New Title"),
                        description: Some("New desc"),
                        body: Some("New body")
                    }
                ))
                .answers(|_| Ok(()))
                .once()
                .in_order(),
            article_db::select::Fn
                .next_call(matching!(
                    UserId(Some(_)),
                    article_db::Filter {
                        slug: Some("new-title"),
                        ..
                    }
                ))
                .answers(|_| Ok(vec![test_db_article()]))
                .once()
                .in_order(),
        ]);
        api::update_article(
            &deps,
            Token::from_token("token"),
            "slug",
            ArticleUpdate {
                title: Some("New Title".to_string()),
                description: Some("New desc".to_string()),
                body: Some("New body".to_string()),
            },
        )
        .await
        .unwrap();
    }
}
