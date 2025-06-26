use crate::{
    errors::product_error::ProductServiceError,
    models::product_model::{CreateProductRequest, CreateProductResponse, GetProductRequest, GetProductsByCategoryRequest, ListProductsResponse, Product, UpdateProductStockRequest},
    repositories::product_repository::ProductRepository,
};
use tracing::info;

pub struct ProductService {
    repository: ProductRepository,
}

impl ProductService {
    pub async fn new() -> Result<Self, ProductServiceError> {
        let repository = ProductRepository::new().await?;
        info!("ProductService initialized");
        Ok(Self { repository })
    }

    pub async fn create_product(
        &self,
        request: CreateProductRequest,
    ) -> Result<CreateProductResponse, ProductServiceError> {
        // Validate input
        self.validate_create_product_request(&request)?;

        let product = Product::new(
            request.name,
            request.description,
            request.price,
            request.category,
            request.stock_quantity,
        );
        let created_product = self.repository.create_product(product).await?;

        Ok(CreateProductResponse {
            id: created_product.id.to_string(),
            message: format!("Product created successfully with id: {}", created_product.id),
        })
    }

    pub async fn get_product(&self, request: GetProductRequest) -> Result<Product, ProductServiceError> {
        if request.id.trim().is_empty() {
            return Err(ProductServiceError::Validation {
                message: "Product ID cannot be empty".to_string(),
            });
        }

        self.repository.get_product(&request.id).await
    }

    pub async fn list_products(&self) -> Result<ListProductsResponse, ProductServiceError> {
        let products = self.repository.list_products().await?;
        let total = products.len();

        Ok(ListProductsResponse { products, total })
    }

    pub async fn get_products_by_category(&self, request: GetProductsByCategoryRequest) -> Result<ListProductsResponse, ProductServiceError> {
        if request.category.trim().is_empty() {
            return Err(ProductServiceError::Validation {
                message: "Category cannot be empty".to_string(),
            });
        }

        let products = self.repository.get_products_by_category(&request.category).await?;
        let total = products.len();

        Ok(ListProductsResponse { products, total })
    }

    pub async fn update_product_stock(&self, request: UpdateProductStockRequest) -> Result<Product, ProductServiceError> {
        if request.id.trim().is_empty() {
            return Err(ProductServiceError::Validation {
                message: "Product ID cannot be empty".to_string(),
            });
        }

        if request.quantity < 0 {
            return Err(ProductServiceError::Validation {
                message: "Stock quantity cannot be negative".to_string(),
            });
        }

        self.repository.update_product_stock(&request.id, request.quantity).await
    }

    fn validate_create_product_request(
        &self,
        request: &CreateProductRequest,
    ) -> Result<(), ProductServiceError> {
        if request.name.trim().is_empty() {
            return Err(ProductServiceError::Validation {
                message: "Product name cannot be empty".to_string(),
            });
        }

        if request.description.trim().is_empty() {
            return Err(ProductServiceError::Validation {
                message: "Product description cannot be empty".to_string(),
            });
        }

        if request.price <= 0.0 {
            return Err(ProductServiceError::InvalidPrice {
                price: request.price,
            });
        }

        if request.category.trim().is_empty() {
            return Err(ProductServiceError::Validation {
                message: "Product category cannot be empty".to_string(),
            });
        }

        if request.stock_quantity < 0 {
            return Err(ProductServiceError::Validation {
                message: "Stock quantity cannot be negative".to_string(),
            });
        }

        Ok(())
    }
}
