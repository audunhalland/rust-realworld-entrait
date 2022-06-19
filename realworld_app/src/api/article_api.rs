use crate::article;
use crate::auth::Token;
use realworld_core::error::RwResult;

use axum::extract::{Extension, Path};
use axum::routing::{get, post};
use axum::{Json, Router};

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct ArticleBody<T = article::Article> {
    article: T,
}

#[derive(serde::Serialize)]
// Just trying this out to avoid the tautology of `ArticleBody<Article>`
struct MultipleArticlesBody {
    articles: Vec<article::Article>,
}

#[derive(serde::Deserialize, serde::Serialize)]
// The Realworld spec doesn't mention this as an API convention, it just finally shows up
// when you're looking at the spec for the Article object and see `tagList` as a field name.
#[serde(rename_all = "camelCase")]
struct CreateArticle {
    title: String,
    description: String,
    body: String,
    tag_list: Vec<String>,
}

#[derive(serde::Deserialize)]
struct UpdateArticle {
    title: Option<String>,
    description: Option<String>,
    body: Option<String>,
}

pub struct ArticleApi<D>(std::marker::PhantomData<D>);

impl<A> ArticleApi<A>
where
    A: Sized + Clone + Send + Sync + 'static,
{
    pub fn router() -> Router {
        Router::new()
            .route(
                "/articles",
                get(Self::list_articles).post(Self::create_article),
            )
            .route(
                "/articles/:slug",
                get(Self::get_article)
                    .put(Self::update_article)
                    .delete(Self::delete_article),
            )
            .route(
                "/api/articles/:slug/favorite",
                post(Self::favorite_article).delete(Self::unfavorite_article),
            )
    }

    async fn list_articles(
        Extension(app): Extension<A>,
        token: Option<Token>,
    ) -> RwResult<Json<MultipleArticlesBody>> {
        todo!()
    }

    async fn get_article(
        Extension(app): Extension<A>,
        token: Option<Token>,
        Path(slug): Path<String>,
    ) -> RwResult<Json<MultipleArticlesBody>> {
        todo!()
    }

    async fn create_article(
        Extension(app): Extension<A>,
        token: Token,
        Json(body): Json<ArticleBody<CreateArticle>>,
    ) -> RwResult<Json<ArticleBody<CreateArticle>>> {
        todo!()
    }

    async fn update_article(
        Extension(app): Extension<A>,
        token: Token,
        Path(slug): Path<String>,
        Json(body): Json<ArticleBody<UpdateArticle>>,
    ) {
        todo!()
    }

    async fn delete_article(Extension(app): Extension<A>, token: Token, Path(slug): Path<String>) {
        todo!()
    }

    async fn favorite_article(
        Extension(app): Extension<A>,
        token: Token,
        Path(slug): Path<String>,
    ) -> RwResult<Json<ArticleBody>> {
        todo!()
    }

    async fn unfavorite_article(
        Extension(app): Extension<A>,
        token: Token,
        Path(slug): Path<String>,
    ) -> RwResult<Json<ArticleBody>> {
        todo!()
    }
}
