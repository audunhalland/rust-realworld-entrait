use realworld_core::error::{RwError, RwResult};
use realworld_core::{GetConfig, System, UserId};

use axum::http::HeaderValue;
use axum::TypedHeader;
use entrait::entrait_export as entrait;
use headers::authorization::Credentials;
use headers::Authorization;
use jwt::SignWithKey;
use jwt::VerifyWithKey;
use uuid::Uuid;

const DEFAULT_SESSION_LENGTH: time::Duration = time::Duration::weeks(2);

#[derive(serde::Serialize, serde::Deserialize)]
struct AuthUserClaims {
    user_id: Uuid,
    /// Standard JWT `exp` claim.
    exp: i64,
}

#[entrait(pub SignUserId)]
fn sign_user_id(deps: &(impl System + GetConfig), user_id: UserId) -> String {
    AuthUserClaims {
        user_id: user_id.0,
        exp: (deps.get_current_time() + DEFAULT_SESSION_LENGTH).unix_timestamp(),
    }
    .sign_with_key(deps.get_jwt_signing_key())
    .expect("HMAC signing should be infallible")
}

/// Marker/Wrapper type for anything authenticated
#[derive(Clone)]
pub struct Authenticated<T>(pub T);

impl<T> From<Authenticated<T>> for MaybeAuthenticated<T> {
    fn from(authenticated: Authenticated<T>) -> Self {
        Self(Some(authenticated.0))
    }
}

impl<T> From<Option<Authenticated<T>>> for MaybeAuthenticated<T> {
    fn from(authenticated: Option<Authenticated<T>>) -> Self {
        match authenticated {
            Some(authenticated) => Self(Some(authenticated.0)),
            None => Self(None),
        }
    }
}

#[derive(Clone)]
pub struct MaybeAuthenticated<T>(pub Option<T>);

#[entrait(pub Authenticate)]
fn authenticate(deps: &(impl System + GetConfig), token: Token) -> RwResult<Authenticated<UserId>> {
    let token = token.token();

    let jwt = jwt::Token::<jwt::Header, AuthUserClaims, _>::parse_unverified(token)
        .map_err(|_| RwError::Unauthorized)?;

    let hmac = deps.get_jwt_signing_key();

    let jwt = jwt
        .verify_with_key(hmac)
        .map_err(|_| RwError::Unauthorized)?;
    let (_header, claims) = jwt.into();

    if claims.exp < deps.get_current_time().unix_timestamp() {
        return Err(RwError::Unauthorized);
    }

    Ok(Authenticated(UserId(claims.user_id)))
}

///
/// Data for `Token` authorization scheme.
///
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
    type Rejection = RwError;

    async fn from_request(
        req: &mut axum::extract::RequestParts<B>,
    ) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(token)) =
            TypedHeader::<Authorization<Token>>::from_request(req)
                .await
                .map_err(|_| RwError::Unauthorized)?;

        Ok(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unimock::*;

    #[test]
    fn should_sign_and_authenticate_token() {
        let user_id =
            UserId(uuid::Uuid::parse_str("20a626ba-c7d3-44c7-981a-e880f81c126f").unwrap());
        let deps = mock(Some(realworld_core::test::mock_system_and_config()));
        let token = sign_user_id(&deps, user_id.clone());

        assert_eq!(
            "eyJhbGciOiJIUzM4NCJ9.eyJ1c2VyX2lkIjoiMjBhNjI2YmEtYzdkMy00NGM3LTk4MWEtZTg4MGY4MWMxMjZmIiwiZXhwIjoxMjA5NjAwfQ.u91-bnMtsP2kKhex_lOiam3WkdEfegS3-qs-V06yehzl2Z5WUd4hH7yH7tFh4zSt",
            token
        );

        let Authenticated(result_user_id) =
            authenticate(&deps, Token(format!("Token {token}"))).unwrap();

        assert_eq!(user_id, result_user_id);
    }
}
