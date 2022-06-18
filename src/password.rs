use crate::db::user_db;
use crate::error::*;

use anyhow::Context;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash};
use entrait::unimock_test::*;

#[entrait(pub HashPassword, async_trait=true, unimock=test)]
async fn hash_password<D>(_: &D, password: String) -> AppResult<user_db::PasswordHash> {
    // Argon2 hashing is designed to be computationally intensive,
    // so we need to do this on a blocking thread.
    Ok(
        tokio::task::spawn_blocking(move || -> AppResult<user_db::PasswordHash> {
            let salt = SaltString::generate(rand::thread_rng());
            Ok(user_db::PasswordHash(
                argon2::PasswordHash::generate(Argon2::default(), password, salt.as_str())
                    .map_err(|e| anyhow::anyhow!("failed to generate password hash: {}", e))?
                    .to_string(),
            ))
        })
        .await
        .context("panic when generating password hash")??,
    )
}

#[entrait(pub VerifyPassword, async_trait=true, unimock=test)]
async fn verify_password<D>(
    _: &D,
    password: String,
    password_hash: user_db::PasswordHash,
) -> AppResult<()> {
    tokio::task::spawn_blocking(move || -> AppResult<()> {
        let hash = PasswordHash::new(&password_hash.0)
            .map_err(|e| anyhow::anyhow!("invalid password hash: {}", e))?;

        hash.verify_password(&[&Argon2::default()], password)
            .map_err(|e| match e {
                argon2::password_hash::Error::Password => Error::Unauthorized,
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
        let hash = hash_password(&(), password.clone()).await.unwrap();

        assert!(verify_password(&(), password.clone(), hash.clone())
            .await
            .is_ok());

        assert_matches!(
            verify_password(&(), "wrong_password".to_string(), hash).await,
            Err(Error::Unauthorized)
        );

        assert_matches!(
            verify_password(
                &(),
                password.clone(),
                user_db::PasswordHash("invalid_hash".to_string())
            )
            .await,
            Err(Error::Anyhow(_))
        );
    }
}
