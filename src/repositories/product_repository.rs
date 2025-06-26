use crate::{errors::product_error::ProductServiceError, models::product_model::Product};
use surrealdb::{engine::local::Mem, Surreal};
use tracing::{error, info};

pub struct ProductRepository {
    db: Surreal<surrealdb::engine::local::Db>,
}

impl ProductRepository {
    pub async fn new() -> Result<Self, ProductServiceError> {
        let db = Surreal::new::<Mem>(()).await?;

        // Use a namespace and database
        db.use_ns("product_service").use_db("products").await?;

        info!("Connected to SurrealDB for Product Service");

        Ok(Self { db })
    }

    pub async fn create_product(&self, product: Product) -> Result<Product, ProductServiceError> {
        // Check if product with name already exists
        let existing: Vec<Product> = self
            .db
            .query("SELECT * FROM product WHERE name = $name")
            .bind(("name", &product.name))
            .await?
            .take(0)?;

        if !existing.is_empty() {
            return Err(ProductServiceError::ProductAlreadyExists {
                name: product.name.clone(),
            });
        }

        // Create the product - let SurrealDB generate the ID
        let product_for_creation = product.for_creation();
        let created: Vec<Product> = self.db.create("product").content(product_for_creation).await?;

        match created.into_iter().next() {
            Some(product) => {
                info!("Created product with id: {}", product.id);
                Ok(product)
            }
            None => {
                error!("Failed to create product");
                Err(ProductServiceError::Internal(anyhow::anyhow!(
                    "Failed to create product"
                )))
            }
        }
    }

    pub async fn get_product(&self, id: &str) -> Result<Product, ProductServiceError> {
        let product: Option<Product> = self.db.select(("product", id)).await?;

        match product {
            Some(product) => {
                info!("Retrieved product with id: {}", id);
                Ok(product)
            }
            None => Err(ProductServiceError::ProductNotFound { id: id.to_string() }),
        }
    }

    pub async fn list_products(&self) -> Result<Vec<Product>, ProductServiceError> {
        let products: Vec<Product> = self
            .db
            .query("SELECT * FROM product ORDER BY created_at DESC")
            .await?
            .take(0)?;

        info!("Retrieved {} products", products.len());
        Ok(products)
    }

    pub async fn get_products_by_category(&self, category: &str) -> Result<Vec<Product>, ProductServiceError> {
        let products: Vec<Product> = self
            .db
            .query("SELECT * FROM product WHERE category = $category ORDER BY name")
            .bind(("category", category))
            .await?
            .take(0)?;

        info!("Retrieved {} products in category '{}'", products.len(), category);
        Ok(products)
    }

    pub async fn update_product_stock(&self, id: &str, new_quantity: i32) -> Result<Product, ProductServiceError> {
        // First get the current product
        let product = self.get_product(id).await?;

        // Update the stock quantity
        let updated: Vec<Product> = self
            .db
            .query("UPDATE $id SET stock_quantity = $quantity, updated_at = time::now()")
            .bind(("id", format!("product:{}", id)))
            .bind(("quantity", new_quantity))
            .await?
            .take(0)?;

        match updated.into_iter().next() {
            Some(product) => {
                info!("Updated stock for product {}: new quantity = {}", id, new_quantity);
                Ok(product)
            }
            None => {
                error!("Failed to update product stock");
                Err(ProductServiceError::Internal(anyhow::anyhow!(
                    "Failed to update product stock"
                )))
            }
        }
    }

    pub async fn get_product_by_name(&self, name: &str) -> Result<Option<Product>, ProductServiceError> {
        let products: Vec<Product> = self
            .db
            .query("SELECT * FROM product WHERE name = $name")
            .bind(("name", name))
            .await?
            .take(0)?;

        Ok(products.into_iter().next())
    }
}
