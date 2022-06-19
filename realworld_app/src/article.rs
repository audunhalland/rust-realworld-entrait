use crate::profile;

use time::OffsetDateTime;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Article {
    slug: String,
    title: String,
    description: String,
    body: String,
    tag_list: Vec<String>,
    created_at: OffsetDateTime,
    // Note: the Postman collection included with the spec assumes that this is never null.
    // We prefer to leave it unset unless the row has actually be updated.
    updated_at: OffsetDateTime,
    favorited: bool,
    favorites_count: i64,
    author: profile::Profile,
}
