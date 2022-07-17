use crate::auth;
use crate::profile;
use realworld_core::error::RwResult;
use realworld_core::UserId;

use entrait::*;
use time::OffsetDateTime;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Article {
    slug: String,
    title: String,
    description: String,
    body: String,
    tag_list: Vec<String>,
    created_at: OffsetDateTime,
    // Note: the Postman collection included with the spec assumes that this is never null.
    // We prefer to leave it unset unless the row has actually be updated.
    updated_at: OffsetDateTime,
    favorited: bool,
    favorites_count: i64,
    author: profile::Profile,
}

#[derive(serde::Deserialize, serde::Serialize)]
// The Realworld spec doesn't mention this as an API convention, it just finally shows up
// when you're looking at the spec for the Article object and see `tagList` as a field name.
#[serde(rename_all = "camelCase")]
pub struct ArticleCreation {
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

#[entrait(pub ListArticles)]
pub async fn list_articles<D>(
    _: &D,
    user_id: Option<auth::Authenticated<UserId>>,
    query: ListArticlesQuery,
) -> RwResult<Vec<Article>> {
    todo!()
}

#[entrait(pub GetArticle)]
pub async fn get_article<D>(
    _: &D,
    user_id: Option<auth::Authenticated<UserId>>,
    slug: String,
) -> RwResult<Article> {
    todo!()
}

#[entrait(pub CreateArticle)]
pub async fn create_article<D>(
    _: &D,
    user_id: auth::Authenticated<UserId>,
    article: ArticleCreation,
) -> RwResult<Article> {
    todo!()
}

#[entrait(pub UpdateArticle)]
pub async fn update_article<D>(
    _: &D,
    user_id: auth::Authenticated<UserId>,
    slug: String,
    article: ArticleUpdate,
) -> RwResult<Article> {
    todo!()
}

#[entrait(pub DeleteArticle)]
pub async fn delete_article<D>(
    _: &D,
    user_id: auth::Authenticated<UserId>,
    slug: String,
) -> RwResult<()> {
    todo!()
}

#[entrait(pub FavoriteArticle)]
pub async fn favorite_article<D>(
    _: &D,
    user_id: auth::Authenticated<UserId>,
    slug: String,
) -> RwResult<Article> {
    todo!()
}

#[entrait(pub UnfavoriteArticle)]
pub async fn unfavorite_article<D>(
    _: &D,
    user_id: auth::Authenticated<UserId>,
    slug: String,
) -> RwResult<Article> {
    todo!()
}
