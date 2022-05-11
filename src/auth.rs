use crate::app::{App, GetCurrentTime, GetJwtSigningKey};
use crate::error::Error;
use crate::user::UserId;

use axum::extract::Extension;
use axum::http::header::AUTHORIZATION;
use axum::http::HeaderValue;
use entrait::unimock_test::*;
use implementation::Impl;
use jwt::VerifyWithKey;
use uuid::Uuid;

pub struct Authenticated<T>(pub T);

#[derive(serde::Serialize, serde::Deserialize)]
struct AuthUserClaims {
    user_id: Uuid,
    /// Standard JWT `exp` claim.
    exp: i64,
}

const SCHEME_PREFIX: &str = "Token ";

#[entrait(Authenticate)]
fn authenticate(
    deps: &(impl GetCurrentTime + GetJwtSigningKey),
    auth_header: &HeaderValue,
) -> Result<Authenticated<UserId>, Error> {
    let auth_header = auth_header.to_str().map_err(|_| Error::Unauthorized)?;

    if !auth_header.starts_with(SCHEME_PREFIX) {
        return Err(Error::Unauthorized);
    }

    let token = &auth_header[SCHEME_PREFIX.len()..];

    let jwt = jwt::Token::<jwt::Header, AuthUserClaims, _>::parse_unverified(token)
        .map_err(|_| Error::Unauthorized)?;

    let hmac = deps.get_jwt_signing_key();

    let jwt = jwt.verify_with_key(hmac).map_err(|_| Error::Unauthorized)?;
    let (_header, claims) = jwt.into();

    if claims.exp < deps.get_current_time().unix_timestamp() {
        return Err(Error::Unauthorized);
    }

    Ok(Authenticated(UserId(claims.user_id)))
}

#[async_trait::async_trait]
impl<B: Send> axum::extract::FromRequest<B> for Authenticated<UserId> {
    type Rejection = Error;

    async fn from_request(
        req: &mut axum::extract::RequestParts<B>,
    ) -> Result<Self, Self::Rejection> {
        let Extension(app): Extension<Impl<App>> = Extension::from_request(req)
            .await
            .expect("BUG: App was not added as an extension");

        let auth_header = req
            .headers()
            .get(AUTHORIZATION)
            .ok_or(Error::Unauthorized)?;

        app.authenticate(auth_header)
    }
}
