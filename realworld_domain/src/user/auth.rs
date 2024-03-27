use super::UserId;
use crate::error::{RwError, RwResult};
use crate::{GetConfig, System};

use axum_extra::TypedHeader;
use entrait::entrait_export as entrait;
use headers::authorization::Credentials;
use headers::Authorization;
use http::HeaderValue;
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

#[entrait(pub SignUserId, mock_api=SignUserIdMock)]
fn sign_user_id(deps: &(impl System + GetConfig), user_id: UserId) -> String {
    AuthUserClaims {
        user_id: user_id.0,
        exp: (deps.get_current_time() + DEFAULT_SESSION_LENGTH).unix_timestamp(),
    }
    .sign_with_key(deps.get_jwt_signing_key())
    .expect("HMAC signing should be infallible")
}

#[entrait(pub Authenticate, mock_api=AuthenticateMock)]
pub mod authenticate {
    use super::*;

    pub fn authenticate(deps: &(impl System + GetConfig), token: Token) -> RwResult<UserId> {
        authenticate_inner(deps, token)
    }

    pub fn opt_authenticate(
        deps: &(impl System + GetConfig),
        token: Option<Token>,
    ) -> RwResult<UserId<Option<Uuid>>> {
        Ok(match token {
            Some(token) => UserId(Some(authenticate_inner(deps, token)?.0)),
            None => UserId(None),
        })
    }

    fn authenticate_inner(deps: &(impl System + GetConfig), token: Token) -> RwResult<UserId> {
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

        Ok(UserId(claims.user_id))
    }
}

///
/// Data for `Token` authorization scheme.
///
#[derive(Debug)]
pub struct Token(String);

impl Token {
    pub fn none() -> Option<Token> {
        None
    }

    pub fn from_token(token: &str) -> Self {
        Self(format!("Token {token}"))
    }

    pub fn token(&self) -> &str {
        &self.0.as_str()["Token ".len()..]
    }
}

impl AsRef<str> for Token {
    fn as_ref(&self) -> &str {
        self.token()
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
impl<S> axum::extract::FromRequestParts<S> for Token
where
    S: Send + Sync,
{
    type Rejection = RwError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(token)) =
            TypedHeader::<Authorization<Token>>::from_request_parts(parts, state)
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
        let deps = Unimock::new(crate::test::mock_system_and_config());
        let token = sign_user_id(&deps, user_id.clone());

        assert_eq!(
            "eyJhbGciOiJIUzM4NCJ9.eyJ1c2VyX2lkIjoiMjBhNjI2YmEtYzdkMy00NGM3LTk4MWEtZTg4MGY4MWMxMjZmIiwiZXhwIjoxMjA5NjAwfQ.u91-bnMtsP2kKhex_lOiam3WkdEfegS3-qs-V06yehzl2Z5WUd4hH7yH7tFh4zSt",
            token
        );

        let result_user_id = authenticate::authenticate(&deps, Token::from_token(&token)).unwrap();

        assert_eq!(user_id, result_user_id);
    }
}
