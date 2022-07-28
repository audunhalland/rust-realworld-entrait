use realworld_article;
use realworld_core::error::RwResult;
use realworld_user::auth::Token;

use axum::extract::{Extension, Path, Query};
use axum::routing::{delete, get, post};
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

#[derive(serde::Deserialize, serde::Serialize)]
struct CommentBody<T = realworld_article::Comment> {
    comment: T,
}

#[derive(serde::Serialize)]
struct MultipleCommentsBody {
    comments: Vec<realworld_article::Comment>,
}

#[derive(serde::Deserialize)]
struct AddComment {
    body: String,
}

pub struct ArticleRoutes<D>(std::marker::PhantomData<D>);

impl<D: Sized + Clone + Send + Sync + 'static> ArticleRoutes<D>
where
    D: realworld_article::List
        + realworld_article::Feed
        + realworld_article::Fetch
        + realworld_article::Create
        + realworld_article::Update
        + realworld_article::Delete
        + realworld_article::Favorite
        + realworld_article::ListComments
        + realworld_article::AddComment
        + realworld_article::DeleteComment,
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
        Query(query): Query<realworld_article::ListArticlesQuery>,
    ) -> RwResult<Json<MultipleArticlesBody>> {
        Ok(Json(MultipleArticlesBody {
            articles: deps.list(token, query).await?,
        }))
    }

    async fn feed_articles(
        Extension(deps): Extension<D>,
        token: Token,
        Query(query): Query<realworld_article::FeedArticlesQuery>,
    ) -> RwResult<Json<MultipleArticlesBody>> {
        Ok(Json(MultipleArticlesBody {
            articles: deps.feed(token, query).await?,
        }))
    }

    async fn get_article(
        Extension(deps): Extension<D>,
        token: Option<Token>,
        Path(slug): Path<String>,
    ) -> RwResult<Json<ArticleBody>> {
        Ok(Json(ArticleBody {
            article: deps.fetch(token, &slug).await?,
        }))
    }

    async fn create_article(
        Extension(deps): Extension<D>,
        token: Token,
        Json(body): Json<ArticleBody<realworld_article::ArticleCreate>>,
    ) -> RwResult<Json<ArticleBody<realworld_article::Article>>> {
        Ok(Json(ArticleBody {
            article: deps.create(token, body.article).await?,
        }))
    }

    async fn update_article(
        Extension(deps): Extension<D>,
        token: Token,
        Path(slug): Path<String>,
        Json(body): Json<ArticleBody<realworld_article::ArticleUpdate>>,
    ) -> RwResult<Json<ArticleBody>> {
        Ok(Json(ArticleBody {
            article: deps.update(token, &slug, body.article).await?,
        }))
    }

    async fn delete_article(
        Extension(deps): Extension<D>,
        token: Token,
        Path(slug): Path<String>,
    ) -> RwResult<()> {
        deps.delete(token, &slug).await?;
        Ok(())
    }

    async fn favorite_article(
        Extension(deps): Extension<D>,
        token: Token,
        Path(slug): Path<String>,
    ) -> RwResult<Json<ArticleBody>> {
        Ok(Json(ArticleBody {
            article: deps.favorite(token, &slug, true).await?,
        }))
    }

    async fn unfavorite_article(
        Extension(deps): Extension<D>,
        token: Token,
        Path(slug): Path<String>,
    ) -> RwResult<Json<ArticleBody>> {
        Ok(Json(ArticleBody {
            article: deps.favorite(token, &slug, false).await?,
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
            realworld_article::list::Fn
                .next_call(matching! {
                    (None, query) if query == &realworld_article::ListArticlesQuery::default()
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
