use realworld_core;
use realworld_core::error::{RwError, RwResult};

use anyhow::Context;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash};
use entrait::entrait_export as entrait;

#[entrait(pub HashPassword, no_deps)]
async fn hash_password(password: String) -> RwResult<realworld_core::PasswordHash> {
    // Argon2 hashing is designed to be computationally intensive,
    // so we need to do this on a blocking thread.
    tokio::task::spawn_blocking(move || -> RwResult<realworld_core::PasswordHash> {
        let salt = SaltString::generate(rand::thread_rng());
        Ok(realworld_core::PasswordHash(
            argon2::PasswordHash::generate(Argon2::default(), password, salt.as_str())
                .map_err(|e| anyhow::anyhow!("failed to generate password hash: {}", e))?
                .to_string(),
        ))
    })
    .await
    .context("panic when generating password hash")?
}

#[entrait(pub VerifyPassword, no_deps)]
async fn verify_password(
    password: String,
    password_hash: realworld_core::PasswordHash,
) -> RwResult<()> {
    tokio::task::spawn_blocking(move || -> RwResult<()> {
        let hash = PasswordHash::new(&password_hash.0)
            .map_err(|e| anyhow::anyhow!("invalid password hash: {}", e))?;

        hash.verify_password(&[&Argon2::default()], password)
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
        let password = "v3rys3cr3t".to_string();
        let app = entrait::Impl::new(());
        let hash = app.hash_password(password.clone()).await.unwrap();

        assert!(app
            .verify_password(password.clone(), hash.clone())
            .await
            .is_ok());

        assert_matches!(
            app.verify_password("wrong_password".to_string(), hash)
                .await,
            Err(RwError::Unauthorized)
        );

        assert_matches!(
            app.verify_password(
                password.clone(),
                realworld_core::PasswordHash("invalid_hash".to_string())
            )
            .await,
            Err(RwError::Anyhow(_))
        );
    }
}
