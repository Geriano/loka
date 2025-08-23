mod error;

pub mod entities;
pub mod methods;

pub use error::{DbErr, Error, Result};

pub(crate) use error::ValidationError;
