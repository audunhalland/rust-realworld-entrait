use realworld_article;
use realworld_core::error::RwResult;
use realworld_user::auth::{self, Token};

use axum::extract::{Extension, Path, Query};
use axum::routing::{get, post};
use axum::Json;

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct ArticleBody<T = realworld_article::Article> {
    article: T,
}

#[derive(serde::Deserialize, serde::Serialize)]
// Just trying this out to avoid the tautology of `ArticleBody<Article>`
struct MultipleArticlesBody {
    articles: Vec<realworld_article::Article>,
}

pub struct ArticleRoutes<A>(std::marker::PhantomData<A>);

impl<A> ArticleRoutes<A>
where
    A: realworld_article::ListArticles
        + realworld_article::FeedArticles
        + realworld_article::GetArticle
        + realworld_article::CreateArticle
        + realworld_article::UpdateArticle
        + realworld_article::DeleteArticle
        + realworld_article::FavoriteArticle
        + realworld_article::UnfavoriteArticle
        + auth::Authenticate
        + Sized
        + Clone
        + Send
        + Sync
        + 'static,
{
    pub fn router() -> axum::Router {
        axum::Router::new()
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
            .route("/articles/feed", get(Self::feed_articles))
    }

    async fn list_articles(
        Extension(app): Extension<A>,
        token: Option<Token>,
        Query(query): Query<realworld_article::ListArticlesQuery>,
    ) -> RwResult<Json<MultipleArticlesBody>> {
        let opt_user = token.map(|token| app.authenticate(token)).transpose()?;
        Ok(Json(MultipleArticlesBody {
            articles: app.list_articles(opt_user.into(), query).await?,
        }))
    }

    async fn feed_articles(
        Extension(app): Extension<A>,
        token: Token,
        Query(query): Query<realworld_article::FeedArticlesQuery>,
    ) -> RwResult<Json<MultipleArticlesBody>> {
        let user = app.authenticate(token)?;
        Ok(Json(MultipleArticlesBody {
            articles: app.feed_articles(user.into(), query).await?,
        }))
    }

    async fn get_article(
        Extension(app): Extension<A>,
        token: Option<Token>,
        Path(slug): Path<String>,
    ) -> RwResult<Json<ArticleBody>> {
        let opt_user = token.map(|token| app.authenticate(token)).transpose()?;
        Ok(Json(ArticleBody {
            article: app.get_article(opt_user.into(), &slug).await?,
        }))
    }

    async fn create_article(
        Extension(app): Extension<A>,
        token: Token,
        Json(body): Json<ArticleBody<realworld_article::ArticleCreate>>,
    ) -> RwResult<Json<ArticleBody<realworld_article::Article>>> {
        let user_id = app.authenticate(token)?;
        Ok(Json(ArticleBody {
            article: app.create_article(user_id, body.article).await?,
        }))
    }

    async fn update_article(
        Extension(app): Extension<A>,
        token: Token,
        Path(slug): Path<String>,
        Json(body): Json<ArticleBody<realworld_article::ArticleUpdate>>,
    ) -> RwResult<Json<ArticleBody>> {
        let user = app.authenticate(token)?;
        Ok(Json(ArticleBody {
            article: app.update_article(user, &slug, body.article).await?,
        }))
    }

    async fn delete_article(
        Extension(app): Extension<A>,
        token: Token,
        Path(slug): Path<String>,
    ) -> RwResult<()> {
        let user = app.authenticate(token)?;
        app.delete_article(user, &slug).await?;
        Ok(())
    }

    async fn favorite_article(
        Extension(app): Extension<A>,
        token: Token,
        Path(slug): Path<String>,
    ) -> RwResult<Json<ArticleBody>> {
        let user = app.authenticate(token)?;
        Ok(Json(ArticleBody {
            article: app.favorite_article(user, &slug).await?,
        }))
    }

    async fn unfavorite_article(
        Extension(app): Extension<A>,
        token: Token,
        Path(slug): Path<String>,
    ) -> RwResult<Json<ArticleBody>> {
        let user = app.authenticate(token)?;
        Ok(Json(ArticleBody {
            article: app.unfavorite_article(user, &slug).await?,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::*;

    use axum::http::{Request, StatusCode};
    use realworld_user::auth::MaybeAuthenticated;
    use unimock::*;

    fn test_router(deps: Unimock) -> axum::Router {
        ArticleRoutes::<Unimock>::router().layer(Extension(deps))
    }

    #[tokio::test]
    async fn list_articles_should_accept_no_auth() {
        let deps = mock(Some(
            realworld_article::list_articles::Fn
                .next_call(matching! {
                    (MaybeAuthenticated(None), query) if query == &realworld_article::ListArticlesQuery::default()
                })
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
