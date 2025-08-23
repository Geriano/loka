pub use sea_orm::error::DbErr;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    DatabaseError(#[from] DbErr),
    #[error("Validation error: {errors:?}")]
    ValidationError { errors: Vec<ValidationError> },
}

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}
