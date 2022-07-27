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

pub struct ArticleRoutes<D>(std::marker::PhantomData<D>);

impl<D> ArticleRoutes<D>
where
    D: realworld_article::List
        + realworld_article::Feed
        + realworld_article::Fetch
        + realworld_article::Create
        + realworld_article::Update
        + realworld_article::Delete
        + realworld_article::Favorite
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
        Extension(deps): Extension<D>,
        token: Option<Token>,
        Query(query): Query<realworld_article::ListArticlesQuery>,
    ) -> RwResult<Json<MultipleArticlesBody>> {
        let opt_current_user = token.map(|token| deps.authenticate(token)).transpose()?;
        Ok(Json(MultipleArticlesBody {
            articles: deps.list(opt_current_user.into(), query).await?,
        }))
    }

    async fn feed_articles(
        Extension(deps): Extension<D>,
        token: Token,
        Query(query): Query<realworld_article::FeedArticlesQuery>,
    ) -> RwResult<Json<MultipleArticlesBody>> {
        let user = deps.authenticate(token)?;
        Ok(Json(MultipleArticlesBody {
            articles: deps.feed(user.into(), query).await?,
        }))
    }

    async fn get_article(
        Extension(deps): Extension<D>,
        token: Option<Token>,
        Path(slug): Path<String>,
    ) -> RwResult<Json<ArticleBody>> {
        let opt_current_user = token.map(|token| deps.authenticate(token)).transpose()?;
        Ok(Json(ArticleBody {
            article: deps.fetch(opt_current_user.into(), &slug).await?,
        }))
    }

    async fn create_article(
        Extension(deps): Extension<D>,
        token: Token,
        Json(body): Json<ArticleBody<realworld_article::ArticleCreate>>,
    ) -> RwResult<Json<ArticleBody<realworld_article::Article>>> {
        let current_user = deps.authenticate(token)?;
        Ok(Json(ArticleBody {
            article: deps.create(current_user, body.article).await?,
        }))
    }

    async fn update_article(
        Extension(deps): Extension<D>,
        token: Token,
        Path(slug): Path<String>,
        Json(body): Json<ArticleBody<realworld_article::ArticleUpdate>>,
    ) -> RwResult<Json<ArticleBody>> {
        let current_user = deps.authenticate(token)?;
        Ok(Json(ArticleBody {
            article: deps.update(current_user, &slug, body.article).await?,
        }))
    }

    async fn delete_article(
        Extension(deps): Extension<D>,
        token: Token,
        Path(slug): Path<String>,
    ) -> RwResult<()> {
        let current_user = deps.authenticate(token)?;
        deps.delete(current_user, &slug).await?;
        Ok(())
    }

    async fn favorite_article(
        Extension(deps): Extension<D>,
        token: Token,
        Path(slug): Path<String>,
    ) -> RwResult<Json<ArticleBody>> {
        let current_user = deps.authenticate(token)?;
        Ok(Json(ArticleBody {
            article: deps.favorite(current_user, &slug, true).await?,
        }))
    }

    async fn unfavorite_article(
        Extension(deps): Extension<D>,
        token: Token,
        Path(slug): Path<String>,
    ) -> RwResult<Json<ArticleBody>> {
        let current_user = deps.authenticate(token)?;
        Ok(Json(ArticleBody {
            article: deps.favorite(current_user, &slug, false).await?,
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
            realworld_article::list::Fn
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
