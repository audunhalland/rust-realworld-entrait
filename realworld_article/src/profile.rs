#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct Profile {
    pub username: String,
    pub bio: String,
    pub image: Option<String>,
    pub following: bool,
}
