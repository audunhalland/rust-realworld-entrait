pub mod repo;

use crate::error::*;
use crate::iter_util::Single;
use crate::timestamp::Timestamptz;
use crate::user::auth::*;
use crate::user::profile::Profile;
use crate::user::UserId;
use repo::ArticleRepo;

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

impl From<repo::Article> for Article {
    fn from(q: repo::Article) -> Self {
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

#[entrait(pub Api, mock_api=mock)]
pub mod api {
    use super::*;

    pub async fn list_articles(
        deps: &(impl Authenticate + ArticleRepo),
        token: Option<Token>,
        query: ListArticlesQuery,
    ) -> RwResult<Vec<Article>> {
        let current_user_id = deps.opt_authenticate(token)?;
        deps.select_articles(
            current_user_id,
            repo::Filter {
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
        deps: &(impl Authenticate + ArticleRepo),
        token: Token,
        query: FeedArticlesQuery,
    ) -> RwResult<Vec<Article>> {
        let current_user_id = deps.authenticate(token)?;
        deps.select_articles(
            current_user_id.some(),
            repo::Filter {
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
        deps: &(impl Authenticate + ArticleRepo),
        token: Option<Token>,
        slug: &str,
    ) -> RwResult<Article> {
        let current_user_id = deps.opt_authenticate(token)?;
        deps.select_articles(
            current_user_id,
            repo::Filter {
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
        deps: &(impl Authenticate + ArticleRepo),
        token: Token,
        article: ArticleCreate,
    ) -> RwResult<Article> {
        let current_user_id = deps.authenticate(token)?;
        let slug = slugify(&article.title);
        deps.insert_article(
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
        deps: &(impl Authenticate + ArticleRepo),
        token: Token,
        slug: &str,
        article_update: ArticleUpdate,
    ) -> RwResult<Article> {
        let current_user_id = deps.authenticate(token)?;
        let new_slug = article_update.title.as_deref().map(slugify);

        deps.update_article(
            current_user_id,
            slug,
            repo::ArticleUpdate {
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
        deps: &(impl Authenticate + ArticleRepo),
        token: Token,
        slug: &str,
    ) -> RwResult<()> {
        let current_user_id = deps.authenticate(token)?;
        deps.delete_article(current_user_id, slug).await
    }

    pub async fn favorite_article(
        deps: &(impl Authenticate + ArticleRepo),
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

    async fn get_single_article(
        deps: &impl ArticleRepo,
        current_user_id: UserId,
        slug: &str,
    ) -> RwResult<Article> {
        deps.select_articles(
            current_user_id.some(),
            repo::Filter {
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
}

#[cfg(test)]
mod tests {
    use crate::user::auth::authenticate::AuthenticateMock;

    use super::{repo::ArticleRepoMock, *};
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

    fn test_db_article() -> repo::Article {
        repo::Article {
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

    fn mock_authenticate() -> impl unimock::Clause {
        AuthenticateMock::authenticate
            .next_call(matching!(_))
            .returns(Ok(UserId(Uuid::new_v4())))
    }

    fn mock_authenticate_anonymous() -> impl unimock::Clause {
        AuthenticateMock::opt_authenticate
            .next_call(matching!(None))
            .returns(Ok(UserId(None)))
    }

    #[tokio::test]
    async fn create_article_should_slugify() {
        let deps = Unimock::new((
            mock_authenticate(),
            ArticleRepoMock::insert_article
                .next_call(matching!(UserId(_), "my-title", _, _, _, _))
                .returns(Ok(test_db_article())),
        ));
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
        let deps = Unimock::new((
            mock_authenticate_anonymous(),
            ArticleRepoMock::select_articles
                .next_call(matching!(
                    UserId(None),
                    repo::Filter {
                        slug: Some("slug"),
                        ..
                    }
                ))
                .returns(Ok(vec![])),
        ));
        assert_matches!(
            api::fetch_article(&deps, Token::none(), "slug").await,
            Err(RwError::ArticleNotFound)
        );
    }

    #[tokio::test]
    async fn update_article_should_update_slug() {
        let deps = Unimock::new((
            mock_authenticate(),
            ArticleRepoMock::update_article
                .next_call(matching!(
                    UserId(_),
                    "slug",
                    repo::ArticleUpdate {
                        slug: Some("new-title"),
                        title: Some("New Title"),
                        description: Some("New desc"),
                        body: Some("New body")
                    }
                ))
                .returns(Ok(())),
            ArticleRepoMock::select_articles
                .next_call(matching!(
                    UserId(Some(_)),
                    repo::Filter {
                        slug: Some("new-title"),
                        ..
                    }
                ))
                .returns(Ok(vec![test_db_article()])),
        ));
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
