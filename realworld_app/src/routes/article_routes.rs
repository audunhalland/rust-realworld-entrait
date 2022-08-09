use realworld_core::article;
use realworld_core::comment;
use realworld_core::error::RwResult;
use realworld_core::user::auth::Token;

use axum::extract::{Extension, Path, Query};
use axum::routing::{delete, get, post};
use axum::Json;

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct ArticleBody<T = article::Article> {
    article: T,
}

#[derive(serde::Deserialize, serde::Serialize)]
// Just trying this out to avoid the tautology of `ArticleBody<Article>`
struct MultipleArticlesBody {
    articles: Vec<article::Article>,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct CommentBody<T = comment::Comment> {
    comment: T,
}

#[derive(serde::Serialize)]
struct MultipleCommentsBody {
    comments: Vec<comment::Comment>,
}

#[derive(serde::Deserialize)]
struct AddComment {
    body: String,
}

pub struct ArticleRoutes<D>(std::marker::PhantomData<D>);

impl<D: Sized + Clone + Send + Sync + 'static> ArticleRoutes<D>
where
    D: article::Api + comment::Api,
{
    pub fn router() -> axum::Router {
        axum::Router::new().nest(
            "/articles",
            axum::Router::new()
                .route("/", get(Self::list_articles).post(Self::create_article))
                .route(
                    "/:slug",
                    get(Self::get_article)
                        .put(Self::update_article)
                        .delete(Self::delete_article),
                )
                .route(
                    "/:slug/favorite",
                    post(Self::favorite_article).delete(Self::unfavorite_article),
                )
                .route("/feed", get(Self::feed_articles))
                .route(
                    "/:slug/comments",
                    get(Self::list_comments).post(Self::add_comment),
                )
                .route("/:slug/comments/:comment_id", delete(Self::delete_comment)),
        )
    }

    async fn list_articles(
        Extension(deps): Extension<D>,
        token: Option<Token>,
        Query(query): Query<article::ListArticlesQuery>,
    ) -> RwResult<Json<MultipleArticlesBody>> {
        Ok(Json(MultipleArticlesBody {
            articles: deps.list_articles(token, query).await?,
        }))
    }

    async fn feed_articles(
        Extension(deps): Extension<D>,
        token: Token,
        Query(query): Query<article::FeedArticlesQuery>,
    ) -> RwResult<Json<MultipleArticlesBody>> {
        Ok(Json(MultipleArticlesBody {
            articles: deps.feed_articles(token, query).await?,
        }))
    }

    async fn get_article(
        Extension(deps): Extension<D>,
        token: Option<Token>,
        Path(slug): Path<String>,
    ) -> RwResult<Json<ArticleBody>> {
        Ok(Json(ArticleBody {
            article: deps.fetch_article(token, &slug).await?,
        }))
    }

    async fn create_article(
        Extension(deps): Extension<D>,
        token: Token,
        Json(body): Json<ArticleBody<article::ArticleCreate>>,
    ) -> RwResult<Json<ArticleBody<article::Article>>> {
        Ok(Json(ArticleBody {
            article: deps.create_article(token, body.article).await?,
        }))
    }

    async fn update_article(
        Extension(deps): Extension<D>,
        token: Token,
        Path(slug): Path<String>,
        Json(body): Json<ArticleBody<article::ArticleUpdate>>,
    ) -> RwResult<Json<ArticleBody>> {
        Ok(Json(ArticleBody {
            article: deps.update_article(token, &slug, body.article).await?,
        }))
    }

    async fn delete_article(
        Extension(deps): Extension<D>,
        token: Token,
        Path(slug): Path<String>,
    ) -> RwResult<()> {
        deps.delete_article(token, &slug).await?;
        Ok(())
    }

    async fn favorite_article(
        Extension(deps): Extension<D>,
        token: Token,
        Path(slug): Path<String>,
    ) -> RwResult<Json<ArticleBody>> {
        Ok(Json(ArticleBody {
            article: deps.favorite_article(token, &slug, true).await?,
        }))
    }

    async fn unfavorite_article(
        Extension(deps): Extension<D>,
        token: Token,
        Path(slug): Path<String>,
    ) -> RwResult<Json<ArticleBody>> {
        Ok(Json(ArticleBody {
            article: deps.favorite_article(token, &slug, false).await?,
        }))
    }

    async fn list_comments(
        Extension(deps): Extension<D>,
        token: Option<Token>,
        Path(slug): Path<String>,
    ) -> RwResult<Json<MultipleCommentsBody>> {
        Ok(Json(MultipleCommentsBody {
            comments: deps.list_comments(token, &slug).await?,
        }))
    }

    async fn add_comment(
        Extension(deps): Extension<D>,
        token: Token,
        Path(slug): Path<String>,
        Json(CommentBody { comment }): Json<CommentBody<AddComment>>,
    ) -> RwResult<Json<CommentBody>> {
        Ok(Json(CommentBody {
            comment: deps.add_comment(token, &slug, &comment.body).await?,
        }))
    }

    async fn delete_comment(
        Extension(deps): Extension<D>,
        token: Token,
        Path(slug): Path<String>,
        Path(comment_id): Path<i64>,
    ) -> RwResult<()> {
        deps.delete_comment(token, &slug, comment_id).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::*;

    use axum::http::{Request, StatusCode};
    use unimock::*;

    fn test_router(deps: Unimock) -> axum::Router {
        ArticleRoutes::<Unimock>::router().layer(Extension(deps))
    }

    #[tokio::test]
    async fn list_articles_should_accept_no_auth() {
        let deps = mock(Some(
            article::api::list_articles::Fn
                .next_call(matching! {
                    (None, query) if query == &article::ListArticlesQuery::default()
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
