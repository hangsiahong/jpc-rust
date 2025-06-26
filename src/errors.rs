use thiserror::Error;

#[derive(Error, Debug)]
pub enum UserServiceError {
    #[error("Database error: {0}")]
    Database(#[from] surrealdb::Error),
    
    #[error("User not found with id: {id}")]
    UserNotFound { id: String },
    
    #[error("Invalid email format: {email}")]
    InvalidEmail { email: String },
    
    #[error("User already exists with email: {email}")]
    UserAlreadyExists { email: String },
    
    #[error("Validation error: {message}")]
    Validation { message: String },
    
    #[error("Internal server error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl From<UserServiceError> for jsonrpsee::types::ErrorCode {
    fn from(err: UserServiceError) -> Self {
        match err {
            UserServiceError::UserNotFound { .. } => jsonrpsee::types::ErrorCode::InvalidParams,
            UserServiceError::InvalidEmail { .. } => jsonrpsee::types::ErrorCode::InvalidParams,
            UserServiceError::UserAlreadyExists { .. } => jsonrpsee::types::ErrorCode::InvalidParams,
            UserServiceError::Validation { .. } => jsonrpsee::types::ErrorCode::InvalidParams,
            _ => jsonrpsee::types::ErrorCode::InternalError,
        }
    }
}
