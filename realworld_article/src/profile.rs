#[derive(serde::Deserialize, serde::Serialize, Clone)]
#[cfg_attr(test, derive(Debug))]
pub struct Profile {
    pub username: String,
    pub bio: String,
    pub image: Option<String>,
    pub following: bool,
}
