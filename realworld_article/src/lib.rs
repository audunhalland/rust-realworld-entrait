use realworld_core::error::*;
use realworld_core::iter_util::Single;
use realworld_core::timestamp::Timestamptz;
use realworld_core::UserId;
use realworld_db::article_db;
use realworld_user::auth::{Authenticated, MaybeAuthenticated};
use realworld_user::profile::Profile;

use entrait::entrait_export as entrait;
use itertools::Itertools;

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

#[entrait(pub List)]
async fn list(
    deps: &impl article_db::SelectArticles,
    MaybeAuthenticated(current_user_id): MaybeAuthenticated<UserId>,
    query: ListArticlesQuery,
) -> RwResult<Vec<Article>> {
    deps.select_articles(
        UserId(current_user_id.map(UserId::into_id)),
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

#[derive(serde::Deserialize, Default)]
#[serde(default)]
pub struct FeedArticlesQuery {
    // See comment on these fields in `ListArticlesQuery` above.
    limit: Option<i64>,
    offset: Option<i64>,
}

#[entrait(pub Feed)]
async fn feed(
    deps: &impl article_db::SelectArticles,
    Authenticated(current_user_id): Authenticated<UserId>,
    query: FeedArticlesQuery,
) -> RwResult<Vec<Article>> {
    deps.select_articles(
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

#[entrait(pub Fetch)]
async fn fetch(
    deps: &impl article_db::SelectArticles,
    MaybeAuthenticated(current_user_id): MaybeAuthenticated<UserId>,
    slug: &str,
) -> RwResult<Article> {
    deps.select_articles(
        UserId(current_user_id.map(UserId::into_id)),
        article_db::Filter {
            slug: Some(&slug),
            ..Default::default()
        },
    )
    .await?
    .into_iter()
    .single_or_none()?
    .map(Into::into)
    .ok_or(RwError::ArticleNotFound)
}

#[entrait(pub Create)]
async fn create(
    deps: &impl article_db::InsertArticle,
    Authenticated(current_user_id): Authenticated<UserId>,
    article: ArticleCreate,
) -> RwResult<Article> {
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

#[entrait(pub Update)]
async fn update(
    deps: &(impl article_db::UpdateArticle + article_db::SelectArticles),
    Authenticated(current_user_id): Authenticated<UserId>,
    slug: &str,
    article_update: ArticleUpdate,
) -> RwResult<Article> {
    let new_slug = article_update.title.as_deref().map(slugify);

    deps.update_article(
        current_user_id.clone(),
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

#[entrait(pub Delete)]
async fn delete(
    deps: &impl article_db::DeleteArticle,
    Authenticated(current_user_id): Authenticated<UserId>,
    slug: &str,
) -> RwResult<()> {
    deps.delete_article(current_user_id, slug).await
}

#[entrait(pub Favorite)]
async fn favorite(
    deps: &(impl article_db::FavoriteArticle
          + article_db::UnfavoriteArticle
          + article_db::SelectArticles),
    Authenticated(current_user_id): Authenticated<UserId>,
    slug: &str,
    value: bool,
) -> RwResult<Article> {
    if value {
        deps.favorite_article(current_user_id.clone(), slug).await?;
    } else {
        deps.unfavorite_article(current_user_id.clone(), slug)
            .await?;
    }
    get_single_article(deps, current_user_id, slug).await
}

async fn get_single_article(
    deps: &impl article_db::SelectArticles,
    user_id: UserId,
    slug: &str,
) -> RwResult<Article> {
    deps.select_articles(
        user_id.some(),
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

    #[tokio::test]
    async fn create_article_should_slugify() {
        let deps = mock(Some(
            article_db::insert_article::Fn
                .next_call(matching!(UserId(_), "my-title", _, _, _, _))
                .answers(|_| Ok(test_db_article()))
                .once()
                .in_order(),
        ));
        create(
            &deps,
            Authenticated(UserId(uuid::Uuid::new_v4())),
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
        let deps = mock(Some(
            article_db::select_articles::Fn
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
        ));
        assert_matches!(
            fetch(&deps, MaybeAuthenticated(None), "slug").await,
            Err(RwError::ArticleNotFound)
        );
    }

    #[tokio::test]
    async fn update_article_should_update_slug() {
        let deps = mock([
            article_db::update_article::Fn
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
            article_db::select_articles::Fn
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
        update(
            &deps,
            Authenticated(UserId(uuid::Uuid::new_v4())),
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
