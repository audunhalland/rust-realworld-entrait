use crate::app::{GetCurrentTime, GetJwtSigningKey};
use crate::error::Error;
use crate::user::UserId;

use axum::http::HeaderValue;
use axum::TypedHeader;
use entrait::unimock_test::*;
use headers::authorization::Credentials;
use headers::Authorization;
use jwt::VerifyWithKey;
use uuid::Uuid;

/// Marker/Wrapper type for anything authenticated
#[derive(Clone)]
pub struct Authenticated<T>(pub T);

#[entrait(pub Authenticate)]
fn authenticate(
    deps: &(impl GetCurrentTime + GetJwtSigningKey),
    token: Token,
) -> Result<Authenticated<UserId>, Error> {
    let token = token.token();

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

#[derive(Debug)]
pub struct Token(String);

impl Token {
    pub fn token(&self) -> &str {
        &self.0.as_str()["Token ".len()..]
    }
}

impl Credentials for Token {
    const SCHEME: &'static str = "Token";

    fn decode(value: &HeaderValue) -> Option<Self> {
        let auth_header = value.to_str().ok()?;

        Some(Token(auth_header.to_string()))
    }

    fn encode(&self) -> HeaderValue {
        HeaderValue::from_str(&self.0).unwrap()
    }
}

#[async_trait::async_trait]
impl<B: Send> axum::extract::FromRequest<B> for Token {
    type Rejection = Error;

    async fn from_request(
        req: &mut axum::extract::RequestParts<B>,
    ) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(token)) =
            TypedHeader::<Authorization<Token>>::from_request(req)
                .await
                .map_err(|_| Error::Unauthorized)?;

        Ok(token)
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct AuthUserClaims {
    user_id: Uuid,
    /// Standard JWT `exp` claim.
    exp: i64,
}
