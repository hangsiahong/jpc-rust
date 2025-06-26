use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProductServiceError {
    #[error("Database error: {0}")]
    Database(#[from] surrealdb::Error),
    
    #[error("Product not found with id: {id}")]
    ProductNotFound { id: String },
    
    #[error("Invalid price: {price}. Price must be greater than 0")]
    InvalidPrice { price: f64 },
    
    #[error("Product already exists with name: {name}")]
    ProductAlreadyExists { name: String },
    
    #[error("Insufficient stock for product {id}. Available: {available}, Requested: {requested}")]
    InsufficientStock { id: String, available: i32, requested: i32 },
    
    #[error("Validation error: {message}")]
    Validation { message: String },
    
    #[error("Internal server error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl From<ProductServiceError> for jsonrpsee::types::ErrorCode {
    fn from(err: ProductServiceError) -> Self {
        match err {
            ProductServiceError::ProductNotFound { .. } => jsonrpsee::types::ErrorCode::InvalidParams,
            ProductServiceError::InvalidPrice { .. } => jsonrpsee::types::ErrorCode::InvalidParams,
            ProductServiceError::ProductAlreadyExists { .. } => jsonrpsee::types::ErrorCode::InvalidParams,
            ProductServiceError::InsufficientStock { .. } => jsonrpsee::types::ErrorCode::InvalidParams,
            ProductServiceError::Validation { .. } => jsonrpsee::types::ErrorCode::InvalidParams,
            _ => jsonrpsee::types::ErrorCode::InternalError,
        }
    }
}
