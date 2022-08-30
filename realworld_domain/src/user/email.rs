use std::str::FromStr;

use crate::error::RwError;

#[derive(Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, Debug)]
#[serde(transparent)]
pub struct Email(String);

impl Email {
    pub fn valid(email: String) -> Self {
        Self(email)
    }
}

impl FromStr for Email {
    type Err = RwError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // TODO: Email validation?
        Ok(Self(s.into()))
    }
}

impl AsRef<str> for Email {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}
