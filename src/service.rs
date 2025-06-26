use crate::{
    errors::UserServiceError,
    models::{CreateUserRequest, CreateUserResponse, GetUserRequest, ListUsersResponse, User},
    repository::UserRepository,
};
use tracing::info;

pub struct UserService {
    repository: UserRepository,
}

impl UserService {
    pub async fn new() -> Result<Self, UserServiceError> {
        let repository = UserRepository::new().await?;
        info!("UserService initialized");
        Ok(Self { repository })
    }

    pub async fn create_user(
        &self,
        request: CreateUserRequest,
    ) -> Result<CreateUserResponse, UserServiceError> {
        // Validate input
        self.validate_create_user_request(&request)?;

        let user = User::new(request.name, request.email);
        let created_user = self.repository.create_user(user).await?;

        Ok(CreateUserResponse {
            id: created_user.id.to_string(),
            message: format!("User created successfully with id: {}", created_user.id),
        })
    }

    pub async fn get_user(&self, request: GetUserRequest) -> Result<User, UserServiceError> {
        if request.id.trim().is_empty() {
            return Err(UserServiceError::Validation {
                message: "User ID cannot be empty".to_string(),
            });
        }

        self.repository.get_user(&request.id).await
    }

    pub async fn list_users(&self) -> Result<ListUsersResponse, UserServiceError> {
        let users = self.repository.list_users().await?;
        let total = users.len();

        Ok(ListUsersResponse { users, total })
    }

    fn validate_create_user_request(
        &self,
        request: &CreateUserRequest,
    ) -> Result<(), UserServiceError> {
        if request.name.trim().is_empty() {
            return Err(UserServiceError::Validation {
                message: "Name cannot be empty".to_string(),
            });
        }

        if request.email.trim().is_empty() {
            return Err(UserServiceError::Validation {
                message: "Email cannot be empty".to_string(),
            });
        }

        // Simple email validation
        if !request.email.contains('@') || !request.email.contains('.') {
            return Err(UserServiceError::InvalidEmail {
                email: request.email.clone(),
            });
        }

        Ok(())
    }
}
