#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("an error occurred with the database")]
    Sqlx(#[from] sqlx::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
