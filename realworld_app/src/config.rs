#[derive(clap::Parser)]
pub struct Config {
    #[clap(long, env)]
    pub database_url: String,

    #[clap(long, env)]
    pub jwt_signing_key: JtwSigningKey,
}

#[derive(Clone)]
pub struct JtwSigningKey(pub hmac::Hmac<sha2::Sha384>);

impl std::str::FromStr for JtwSigningKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use hmac::Mac;

        Ok(Self(
            hmac::Hmac::<sha2::Sha384>::new_from_slice(s.as_bytes())
                .map_err(|e| format!("Failed to parse hmac: {e:?}"))?,
        ))
    }
}
