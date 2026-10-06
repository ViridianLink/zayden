use crate::admin::{AdminError, LoadoutFieldError};
use crate::form::FieldError;

#[derive(Debug, thiserror::Error)]
pub(super) enum DeleteError {
    #[error(transparent)]
    Form(#[from] FieldError),
    #[error(transparent)]
    Id(#[from] LoadoutFieldError),
    #[error(transparent)]
    Admin(#[from] AdminError),
}
