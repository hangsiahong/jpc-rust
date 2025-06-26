use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Thing,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserForCreation {
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(name: String, email: String) -> Self {
        let now = Utc::now();
        Self {
            id: Thing::from(("user", "temp")), // Will be replaced by SurrealDB
            name,
            email,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn for_creation(&self) -> UserForCreation {
        UserForCreation {
            name: self.name.clone(),
            email: self.email.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    pub fn id_string(&self) -> String {
        self.id.to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserResponse {
    pub id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetUserRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListUsersResponse {
    pub users: Vec<User>,
    pub total: usize,
}
