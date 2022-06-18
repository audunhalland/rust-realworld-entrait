use axum::http::header::WWW_AUTHENTICATE;
use axum::http::StatusCode;
use axum::http::{HeaderMap, HeaderValue};
use axum::response::{IntoResponse, Response};
use axum::Json;
use std::borrow::Cow;
use std::collections::HashMap;

pub type AppResult<T, E = Error> = std::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("authentication required")]
    Unauthorized,

    #[error("email does not exist")]
    EmailDoesNotExist,

    #[error("username is taken")]
    UsernameTaken,

    #[error("email is taken")]
    EmailTaken,

    #[error("an error occurred with the database")]
    Sqlx(#[from] sqlx::Error),

    #[error("an internal server error occurred")]
    Anyhow(#[from] anyhow::Error),
}

impl Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::EmailDoesNotExist => StatusCode::UNPROCESSABLE_ENTITY,
            Self::UsernameTaken => StatusCode::UNPROCESSABLE_ENTITY,
            Self::EmailTaken => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Sqlx(_) | Self::Anyhow(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl axum::response::IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::Unauthorized => (
                self.status_code(),
                [(WWW_AUTHENTICATE, HeaderValue::from_static("Token"))]
                    .into_iter()
                    .collect::<HeaderMap>(),
                self.to_string(),
            )
                .into_response(),
            Self::EmailDoesNotExist => {
                unprocessable_entity_with_errors([("email".into(), vec!["does not exist".into()])])
            }
            Self::UsernameTaken => unprocessable_entity_with_errors([(
                "username".into(),
                vec!["username is taken".into()],
            )]),
            Self::EmailTaken => {
                unprocessable_entity_with_errors([("email".into(), vec!["email is taken".into()])])
            }
            Self::Sqlx(ref e) => {
                // TODO: we probably want to use `tracing` instead
                // so that this gets linked to the HTTP request by `TraceLayer`.
                tracing::error!("SQLx error: {:?}", e);
                (self.status_code(), self.to_string()).into_response()
            }
            Self::Anyhow(ref e) => {
                // TODO: we probably want to use `tracing` instead
                // so that this gets linked to the HTTP request by `TraceLayer`.
                tracing::error!("Generic error: {:?}", e);
                (self.status_code(), self.to_string()).into_response()
            }
        }
    }
}

#[derive(serde::Serialize)]
struct JsonErrors {
    errors: HashMap<Cow<'static, str>, Vec<Cow<'static, str>>>,
}

fn unprocessable_entity_with_errors(
    errors: impl Into<HashMap<Cow<'static, str>, Vec<Cow<'static, str>>>>,
) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(JsonErrors {
            errors: errors.into(),
        }),
    )
        .into_response()
}
