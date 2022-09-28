use crate::error::{RwError, RwResult};

use anyhow::Context;
use argon2::password_hash::SaltString;
use argon2::Argon2;
use entrait::entrait_export as entrait;

/// Warning: This should not implement Debug in production
#[derive(Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct CleartextPassword(pub String);

impl<S: Into<String>> From<S> for CleartextPassword {
    fn from(s: S) -> Self {
        Self(s.into())
    }
}

impl AsRef<str> for CleartextPassword {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PasswordHash(pub String);

impl<S: Into<String>> From<S> for PasswordHash {
    fn from(s: S) -> Self {
        Self(s.into())
    }
}

impl AsRef<str> for PasswordHash {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

#[entrait(pub HashPassword, no_deps, mock_api=HashPasswordMock)]
async fn hash_password(password: CleartextPassword) -> RwResult<PasswordHash> {
    // Argon2 hashing is designed to be computationally intensive,
    // so we need to do this on a blocking thread.
    tokio::task::spawn_blocking(move || -> RwResult<PasswordHash> {
        let salt = SaltString::generate(rand::thread_rng());
        Ok(
            argon2::PasswordHash::generate(Argon2::default(), password.0, salt.as_str())
                .map_err(|e| anyhow::anyhow!("failed to generate password hash: {}", e))?
                .to_string()
                .into(),
        )
    })
    .await
    .context("panic when generating password hash")?
}

#[entrait(pub VerifyPassword, no_deps, mock_api=VerifyPasswordMock)]
async fn verify_password(password: CleartextPassword, password_hash: PasswordHash) -> RwResult<()> {
    tokio::task::spawn_blocking(move || -> RwResult<()> {
        use argon2::password_hash::PasswordHash;
        let hash = PasswordHash::new(&password_hash.0)
            .map_err(|e| anyhow::anyhow!("invalid password hash: {}", e))?;

        hash.verify_password(&[&Argon2::default()], password.0)
            .map_err(|e| match e {
                argon2::password_hash::Error::Password => RwError::Unauthorized,
                _ => anyhow::anyhow!("failed to verify password hash: {}", e).into(),
            })
    })
    .await
    .context("panic when verifying password hash")??;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::*;

    #[tokio::test]
    async fn password_hashing_should_work() {
        let password = CleartextPassword("v3rys3cr3t".to_string());
        let app = entrait::Impl::new(());
        let hash = app.hash_password(password.clone()).await.unwrap();

        assert!(app
            .verify_password(password.clone(), hash.clone())
            .await
            .is_ok());

        assert_matches!(
            app.verify_password("wrong_password".into(), hash).await,
            Err(RwError::Unauthorized)
        );

        assert_matches!(
            app.verify_password(password.clone(), "invalid_hash_format".into())
                .await,
            Err(RwError::Anyhow(_))
        );
    }
}
