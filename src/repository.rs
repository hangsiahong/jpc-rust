use crate::{errors::UserServiceError, models::User};
use surrealdb::{engine::local::Mem, Surreal};
use tracing::{error, info};

pub struct UserRepository {
    db: Surreal<surrealdb::engine::local::Db>,
}

impl UserRepository {
    pub async fn new() -> Result<Self, UserServiceError> {
        let db = Surreal::new::<Mem>(()).await?;

        // Use a namespace and database
        db.use_ns("user_service").use_db("users").await?;

        info!("Connected to SurrealDB");

        Ok(Self { db })
    }

    pub async fn create_user(&self, user: User) -> Result<User, UserServiceError> {
        // Check if user with email already exists
        let existing: Vec<User> = self
            .db
            .query("SELECT * FROM user WHERE email = $email")
            .bind(("email", &user.email))
            .await?
            .take(0)?;

        if !existing.is_empty() {
            return Err(UserServiceError::UserAlreadyExists {
                email: user.email.clone(),
            });
        }

        // Create the user - let SurrealDB generate the ID
        let user_for_creation = user.for_creation();
        let created: Vec<User> = self.db.create("user").content(user_for_creation).await?;

        match created.into_iter().next() {
            Some(user) => {
                info!("Created user with id: {}", user.id);
                Ok(user)
            }
            None => {
                error!("Failed to create user");
                Err(UserServiceError::Internal(anyhow::anyhow!(
                    "Failed to create user"
                )))
            }
        }
    }

    pub async fn get_user(&self, id: &str) -> Result<User, UserServiceError> {
        let user: Option<User> = self.db.select(("user", id)).await?;

        match user {
            Some(user) => {
                info!("Retrieved user with id: {}", id);
                Ok(user)
            }
            None => Err(UserServiceError::UserNotFound { id: id.to_string() }),
        }
    }

    pub async fn list_users(&self) -> Result<Vec<User>, UserServiceError> {
        let users: Vec<User> = self
            .db
            .query("SELECT * FROM user ORDER BY created_at DESC")
            .await?
            .take(0)?;

        info!("Retrieved {} users", users.len());
        Ok(users)
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, UserServiceError> {
        let users: Vec<User> = self
            .db
            .query("SELECT * FROM user WHERE email = $email")
            .bind(("email", email))
            .await?
            .take(0)?;

        Ok(users.into_iter().next())
    }
}
