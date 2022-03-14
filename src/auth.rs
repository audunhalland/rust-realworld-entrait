use axum::body::Body;

use crate::error::Error;

pub struct Auth {
    user_id: uuid::Uuid,
}

#[async_trait::async_trait]
impl<B: Send> axum::extract::FromRequest<B> for Auth {
    type Rejection = Error;

    async fn from_request(
        _req: &mut axum::extract::RequestParts<B>,
    ) -> Result<Self, Self::Rejection> {
        panic!()
    }
}
