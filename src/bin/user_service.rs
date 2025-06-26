use jpc_rust::{
    errors::user_error::UserServiceError,
    models::user_model::{
        CreateUserRequest, CreateUserResponse, GetUserRequest, ListUsersResponse, User,
    },
    services::user_service::UserService,
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
pub trait UserRpc {
    #[method(name = "create_user")]
    async fn create_user(&self, request: CreateUserRequest) -> RpcResult<CreateUserResponse>;

    #[method(name = "get_user")]
    async fn get_user(&self, request: GetUserRequest) -> RpcResult<User>;

    #[method(name = "list_users")]
    async fn list_users(&self) -> RpcResult<ListUsersResponse>;

    #[method(name = "health")]
    async fn health(&self) -> RpcResult<String>;
}

pub struct UserRpcImpl {
    service: Arc<RwLock<UserService>>,
}

impl UserRpcImpl {
    pub async fn new() -> Result<Self, UserServiceError> {
        let service = UserService::new().await?;
        Ok(Self {
            service: Arc::new(RwLock::new(service)),
        })
    }
}

#[async_trait]
impl UserRpcServer for UserRpcImpl {
    async fn create_user(&self, request: CreateUserRequest) -> RpcResult<CreateUserResponse> {
        info!("Creating user: {:?}", request);

        let service = self.service.read().await;
        match service.create_user(request).await {
            Ok(response) => {
                info!("User created successfully: {}", response.id);
                Ok(response)
            }
            Err(err) => {
                error!("Failed to create user: {}", err);
                Err(ErrorObject::owned(
                    ErrorCode::InternalError.code(),
                    "Failed to create user",
                    Some(err.to_string()),
                ))
            }
        }
    }

    async fn get_user(&self, request: GetUserRequest) -> RpcResult<User> {
        info!("Getting user: {:?}", request);

        let service = self.service.read().await;
        match service.get_user(request).await {
            Ok(user) => {
                info!("User retrieved successfully: {}", user.id);
                Ok(user)
            }
            Err(err) => {
                error!("Failed to get user: {}", err);
                Err(ErrorObject::owned(
                    ErrorCode::InternalError.code(),
                    "Failed to get user",
                    Some(err.to_string()),
                ))
            }
        }
    }

    async fn list_users(&self) -> RpcResult<ListUsersResponse> {
        info!("Listing users");

        let service = self.service.read().await;
        match service.list_users().await {
            Ok(response) => {
                info!("Users listed successfully: {} users", response.total);
                Ok(response)
            }
            Err(err) => {
                error!("Failed to list users: {}", err);
                Err(ErrorObject::owned(
                    ErrorCode::InternalError.code(),
                    "Failed to list users",
                    Some(err.to_string()),
                ))
            }
        }
    }

    async fn health(&self) -> RpcResult<String> {
        Ok("User Service is healthy!".to_string())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Starting User Service...");

    // Create the RPC service
    let user_rpc = UserRpcImpl::new().await?;

    // Build the server
    let server = ServerBuilder::default().build("127.0.0.1:8080").await?;

    // Register the methods
    let handle = server.start(user_rpc.into_rpc());

    info!("🚀 User Service started on http://127.0.0.1:8080");
    info!("Available methods:");
    info!("  - create_user(name: String, email: String)");
    info!("  - get_user(id: String)");
    info!("  - list_users()");
    info!("  - health()");

    // Set up graceful shutdown handling
    let handle_clone = handle.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for ctrl+c");
        info!("Received shutdown signal, gracefully shutting down...");
        handle_clone.stop().unwrap();
    });

    // Wait for the server to finish
    handle.stopped().await;
    info!("User Service shut down gracefully");

    Ok(())
}
