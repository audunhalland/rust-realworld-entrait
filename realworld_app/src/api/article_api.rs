use crate::article;
use crate::auth::{self, Token};
use realworld_core::error::RwResult;

use axum::extract::{Extension, Path, Query};
use axum::routing::{get, post};
use axum::{Json, Router};

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct ArticleBody<T = article::Article> {
    article: T,
}

#[derive(serde::Deserialize, serde::Serialize)]
// Just trying this out to avoid the tautology of `ArticleBody<Article>`
struct MultipleArticlesBody {
    articles: Vec<article::Article>,
}

#[derive(serde::Deserialize, Default)]
#[serde(default)]
pub struct FeedArticlesQuery {
    // See comment on these fields in `ListArticlesQuery` above.
    limit: Option<i64>,
    offset: Option<i64>,
}

pub struct ArticleApi<D>(std::marker::PhantomData<D>);

impl<A> ArticleApi<A>
where
    A: article::ListArticles
        + article::GetArticle
        + article::CreateArticle
        + article::UpdateArticle
        + article::DeleteArticle
        + article::FavoriteArticle
        + article::UnfavoriteArticle
        + auth::Authenticate
        + Sized
        + Clone
        + Send
        + Sync
        + 'static,
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
                "/articles/:slug/favorite",
                post(Self::favorite_article).delete(Self::unfavorite_article),
            )
    }

    async fn list_articles(
        Extension(app): Extension<A>,
        token: Option<Token>,
        Query(query): Query<article::ListArticlesQuery>,
    ) -> RwResult<Json<MultipleArticlesBody>> {
        let user_id = token.map(|token| app.authenticate(token)).transpose()?;
        Ok(Json(MultipleArticlesBody {
            articles: app.list_articles(user_id, query).await?,
        }))
    }

    async fn get_article(
        Extension(app): Extension<A>,
        token: Option<Token>,
        Path(slug): Path<String>,
    ) -> RwResult<Json<ArticleBody>> {
        let user_id = token.map(|token| app.authenticate(token)).transpose()?;
        Ok(Json(ArticleBody {
            article: app.get_article(user_id, slug).await?,
        }))
    }

    async fn create_article(
        Extension(app): Extension<A>,
        token: Token,
        Json(body): Json<ArticleBody<article::ArticleCreation>>,
    ) -> RwResult<Json<ArticleBody<article::Article>>> {
        let user_id = app.authenticate(token)?;
        Ok(Json(ArticleBody {
            article: app.create_article(user_id, body.article).await?,
        }))
    }

    async fn update_article(
        Extension(app): Extension<A>,
        token: Token,
        Path(slug): Path<String>,
        Json(body): Json<ArticleBody<article::ArticleUpdate>>,
    ) -> RwResult<Json<ArticleBody>> {
        let user_id = app.authenticate(token)?;
        Ok(Json(ArticleBody {
            article: app.update_article(user_id, slug, body.article).await?,
        }))
    }

    async fn delete_article(
        Extension(app): Extension<A>,
        token: Token,
        Path(slug): Path<String>,
    ) -> RwResult<()> {
        let user_id = app.authenticate(token)?;
        app.delete_article(user_id, slug).await?;
        Ok(())
    }

    async fn favorite_article(
        Extension(app): Extension<A>,
        token: Token,
        Path(slug): Path<String>,
    ) -> RwResult<Json<ArticleBody>> {
        let user_id = app.authenticate(token)?;
        Ok(Json(ArticleBody {
            article: app.favorite_article(user_id, slug).await?,
        }))
    }

    async fn unfavorite_article(
        Extension(app): Extension<A>,
        token: Token,
        Path(slug): Path<String>,
    ) -> RwResult<Json<ArticleBody>> {
        let user_id = app.authenticate(token)?;
        Ok(Json(ArticleBody {
            article: app.unfavorite_article(user_id, slug).await?,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::*;

    use axum::http::{Request, StatusCode};
    use unimock::*;

    fn test_router(deps: Unimock) -> Router {
        ArticleApi::<Unimock>::router().layer(Extension(deps.clone()))
    }

    #[tokio::test]
    async fn list_articles_should_accept_no_auth() {
        let deps = mock(Some(
            article::list_articles::Fn::next_call(
                matching!((None, q) if q == &article::ListArticlesQuery::default()),
            )
            .answers(|_| Ok(vec![]))
            .once()
            .in_order(),
        ));

        let (status, body) = request_json::<MultipleArticlesBody>(
            test_router(deps.clone()),
            Request::get("/articles").empty_body(),
        )
        .await
        .unwrap();

        assert_eq!(StatusCode::OK, status);
        assert!(body.articles.is_empty());
    }
}
