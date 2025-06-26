use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: Thing,
    pub name: String,
    pub description: String,
    pub price: f64,
    pub category: String,
    pub stock_quantity: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductForCreation {
    pub name: String,
    pub description: String,
    pub price: f64,
    pub category: String,
    pub stock_quantity: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Product {
    pub fn new(name: String, description: String, price: f64, category: String, stock_quantity: i32) -> Self {
        let now = Utc::now();
        Self {
            id: Thing::from(("product", "temp")), // Will be replaced by SurrealDB
            name,
            description,
            price,
            category,
            stock_quantity,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn for_creation(&self) -> ProductForCreation {
        ProductForCreation {
            name: self.name.clone(),
            description: self.description.clone(),
            price: self.price,
            category: self.category.clone(),
            stock_quantity: self.stock_quantity,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    pub fn id_string(&self) -> String {
        self.id.to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProductRequest {
    pub name: String,
    pub description: String,
    pub price: f64,
    pub category: String,
    pub stock_quantity: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProductResponse {
    pub id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetProductRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProductStockRequest {
    pub id: String,
    pub quantity: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProductsResponse {
    pub products: Vec<Product>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetProductsByCategoryRequest {
    pub category: String,
}
