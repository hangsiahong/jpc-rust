use jpc_rust::{
    errors::product_error::ProductServiceError,
    models::product_model::{
        CreateProductRequest, CreateProductResponse, GetProductRequest, GetProductsByCategoryRequest,
        ListProductsResponse, Product, UpdateProductStockRequest,
    },
    services::product_service::ProductService,
};
use jsonrpsee::{
    core::{async_trait, RpcResult},
    proc_macros::rpc,
    server::ServerBuilder,
    types::{ErrorCode, ErrorObject},
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, Level};
use tracing_subscriber;

#[rpc(server)]
pub trait ProductRpc {
    #[method(name = "create_product")]
    async fn create_product(&self, request: CreateProductRequest) -> RpcResult<CreateProductResponse>;

    #[method(name = "get_product")]
    async fn get_product(&self, request: GetProductRequest) -> RpcResult<Product>;

    #[method(name = "list_products")]
    async fn list_products(&self) -> RpcResult<ListProductsResponse>;

    #[method(name = "get_products_by_category")]
    async fn get_products_by_category(&self, request: GetProductsByCategoryRequest) -> RpcResult<ListProductsResponse>;

    #[method(name = "update_product_stock")]
    async fn update_product_stock(&self, request: UpdateProductStockRequest) -> RpcResult<Product>;

    #[method(name = "health")]
    async fn health(&self) -> RpcResult<String>;
}

pub struct ProductRpcImpl {
    service: Arc<RwLock<ProductService>>,
}

impl ProductRpcImpl {
    pub async fn new() -> Result<Self, ProductServiceError> {
        let service = ProductService::new().await?;
        Ok(Self {
            service: Arc::new(RwLock::new(service)),
        })
    }
}

#[async_trait]
impl ProductRpcServer for ProductRpcImpl {
    async fn create_product(&self, request: CreateProductRequest) -> RpcResult<CreateProductResponse> {
        info!("Creating product: {:?}", request);

        let service = self.service.read().await;
        match service.create_product(request).await {
            Ok(response) => {
                info!("Product created successfully: {}", response.id);
                Ok(response)
            }
            Err(err) => {
                error!("Failed to create product: {}", err);
                Err(ErrorObject::owned(
                    ErrorCode::InternalError.code(),
                    "Failed to create product",
                    Some(err.to_string()),
                ))
            }
        }
    }

    async fn get_product(&self, request: GetProductRequest) -> RpcResult<Product> {
        info!("Getting product: {:?}", request);

        let service = self.service.read().await;
        match service.get_product(request).await {
            Ok(product) => {
                info!("Product retrieved successfully: {}", product.id);
                Ok(product)
            }
            Err(err) => {
                error!("Failed to get product: {}", err);
                Err(ErrorObject::owned(
                    ErrorCode::InternalError.code(),
                    "Failed to get product",
                    Some(err.to_string()),
                ))
            }
        }
    }

    async fn list_products(&self) -> RpcResult<ListProductsResponse> {
        info!("Listing products");

        let service = self.service.read().await;
        match service.list_products().await {
            Ok(response) => {
                info!("Products listed successfully: {} products", response.total);
                Ok(response)
            }
            Err(err) => {
                error!("Failed to list products: {}", err);
                Err(ErrorObject::owned(
                    ErrorCode::InternalError.code(),
                    "Failed to list products",
                    Some(err.to_string()),
                ))
            }
        }
    }

    async fn get_products_by_category(&self, request: GetProductsByCategoryRequest) -> RpcResult<ListProductsResponse> {
        info!("Getting products by category: {:?}", request);

        let service = self.service.read().await;
        match service.get_products_by_category(request).await {
            Ok(response) => {
                info!("Products by category retrieved successfully: {} products", response.total);
                Ok(response)
            }
            Err(err) => {
                error!("Failed to get products by category: {}", err);
                Err(ErrorObject::owned(
                    ErrorCode::InternalError.code(),
                    "Failed to get products by category",
                    Some(err.to_string()),
                ))
            }
        }
    }

    async fn update_product_stock(&self, request: UpdateProductStockRequest) -> RpcResult<Product> {
        info!("Updating product stock: {:?}", request);

        let service = self.service.read().await;
        match service.update_product_stock(request).await {
            Ok(product) => {
                info!("Product stock updated successfully: {}", product.id);
                Ok(product)
            }
            Err(err) => {
                error!("Failed to update product stock: {}", err);
                Err(ErrorObject::owned(
                    ErrorCode::InternalError.code(),
                    "Failed to update product stock",
                    Some(err.to_string()),
                ))
            }
        }
    }

    async fn health(&self) -> RpcResult<String> {
        Ok("Product Service is healthy!".to_string())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Starting Product Service...");

    // Create the RPC service
    let product_rpc = ProductRpcImpl::new().await?;

    // Build the server on a different port than user service
    let server = ServerBuilder::default().build("127.0.0.1:8081").await?;

    // Register the methods
    let handle = server.start(product_rpc.into_rpc());

    info!("🚀 Product Service started on http://127.0.0.1:8081");
    info!("Available methods:");
    info!("  - create_product(name: String, description: String, price: f64, category: String, stock_quantity: i32)");
    info!("  - get_product(id: String)");
    info!("  - list_products()");
    info!("  - get_products_by_category(category: String)");
    info!("  - update_product_stock(id: String, quantity: i32)");
    info!("  - health()");

    // Wait for the server to finish
    handle.stopped().await;

    Ok(())
}
