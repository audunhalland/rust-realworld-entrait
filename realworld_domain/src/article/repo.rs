use super::UserId;
use crate::{error::RwResult, timestamp::Timestamptz};

use entrait::entrait_export as entrait;

#[derive(Eq, PartialEq, Debug)]
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
    pub favorited_by: Option<&'a str>,
    pub followed_by: Option<UserId>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Default)]
pub struct ArticleUpdate<'a> {
    pub slug: Option<&'a str>,
    pub title: Option<&'a str>,
    pub description: Option<&'a str>,
    pub body: Option<&'a str>,
}

#[entrait(ArticleRepoImpl, delegate_by=DelegateArticleRepo, mock_api=ArticleRepoMock)]
pub trait ArticleRepo {
    async fn select_articles(
        &self,
        current_user: UserId<Option<uuid::Uuid>>,
        filter: Filter<'_>,
    ) -> RwResult<Vec<Article>>;

    async fn fetch_article_id(&self, slug: &str) -> RwResult<uuid::Uuid>;

    async fn insert_article(
        &self,
        user_id: UserId,
        slug: &str,
        title: &str,
        description: &str,
        body: &str,
        tag_list: &[String],
    ) -> RwResult<Article>;

    async fn update_article(
        &self,
        user_id: UserId,
        slug: &str,
        up: ArticleUpdate<'_>,
    ) -> RwResult<()>;

    async fn delete_article(&self, user_id: UserId, slug: &str) -> RwResult<()>;

    async fn insert_favorite(&self, user_id: UserId, slug: &str) -> RwResult<()>;

    async fn delete_favorite(&self, user_id: UserId, slug: &str) -> RwResult<()>;
}
