use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Incoming, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::time::{sleep, timeout};
use tracing::{error, info, warn};

type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

#[derive(Debug, Clone)]
struct ServiceHealth {
    is_healthy: bool,
    last_check: Instant,
    consecutive_failures: u32,
}

impl Default for ServiceHealth {
    fn default() -> Self {
        Self {
            is_healthy: true,
            last_check: Instant::now(),
            consecutive_failures: 0,
        }
    }
}

#[derive(Debug)]
struct HealthChecker {
    user_service: Arc<RwLock<ServiceHealth>>,
    product_service: Arc<RwLock<ServiceHealth>>,
}

impl HealthChecker {
    fn new() -> Self {
        Self {
            user_service: Arc::new(RwLock::new(ServiceHealth::default())),
            product_service: Arc::new(RwLock::new(ServiceHealth::default())),
        }
    }

    async fn start_health_checks(&self) {
        let user_health = Arc::clone(&self.user_service);
        let product_health = Arc::clone(&self.product_service);

        // Spawn health check tasks
        tokio::spawn(async move {
            loop {
                Self::check_service_health(&user_health, 8080, "User Service").await;
                sleep(Duration::from_secs(30)).await;
            }
        });

        tokio::spawn(async move {
            loop {
                Self::check_service_health(&product_health, 8081, "Product Service").await;
                sleep(Duration::from_secs(30)).await;
            }
        });
    }

    async fn check_service_health(
        health: &Arc<RwLock<ServiceHealth>>,
        port: u16,
        service_name: &str,
    ) {
        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();

        let health_check_req = Request::builder()
            .method("POST")
            .uri(format!("http://127.0.0.1:{}", port))
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(
                r#"{"jsonrpc":"2.0","method":"health","id":0}"#,
            )))
            .unwrap();

        let is_healthy =
            match timeout(Duration::from_secs(5), client.request(health_check_req)).await {
                Ok(Ok(response)) => response.status().is_success(),
                _ => false,
            };

        let mut health_guard = health.write().await;
        let was_healthy = health_guard.is_healthy;

        if is_healthy {
            if !was_healthy {
                info!("✅ {} is back online!", service_name);
            }
            health_guard.is_healthy = true;
            health_guard.consecutive_failures = 0;
        } else {
            health_guard.consecutive_failures += 1;
            if was_healthy {
                warn!(
                    "❌ {} is down (failure #{})",
                    service_name, health_guard.consecutive_failures
                );
            }
            // Mark as unhealthy after 3 consecutive failures
            if health_guard.consecutive_failures >= 3 {
                health_guard.is_healthy = false;
            }
        }

        health_guard.last_check = Instant::now();
    }

    async fn is_service_healthy(&self, service: &TargetService) -> bool {
        let health = match service {
            TargetService::UserService => &self.user_service,
            TargetService::ProductService => &self.product_service,
        };

        health.read().await.is_healthy
    }
}

async fn handle_request(req: Request<Incoming>) -> Result<Response<BoxBody>, Infallible> {
    info!("Handling request: {} {}", req.method(), req.uri());

    // Handle CORS preflight
    if req.method() == Method::OPTIONS {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
            .header("Access-Control-Allow-Headers", "Content-Type")
            .body(empty_body())
            .unwrap());
    }

    // Route requests based on path
    let path = req.uri().path();
    let target_service = determine_target_service(path);

    // Check service health before proxying
    let health_checker = HEALTH_CHECKER.get().unwrap();
    if !health_checker.is_service_healthy(&target_service).await {
        return Ok(Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .header("Access-Control-Allow-Origin", "*")
            .body(full_body("Service unavailable"))
            .unwrap());
    }

    match proxy_request_with_retry(req, target_service).await {
        Ok(response) => Ok(response),
        Err(err) => {
            error!("Proxy error: {}", err);
            Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Access-Control-Allow-Origin", "*")
                .body(full_body(format!("Proxy error: {}", err)))
                .unwrap())
        }
    }
}

async fn proxy_request_with_retry(
    req: Request<Incoming>,
    target_service: TargetService,
) -> Result<Response<BoxBody>, Box<dyn std::error::Error + Send + Sync>> {
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY_MS: u64 = 100;

    // Extract request parts before consuming the body
    let method = req.method().clone();
    let uri = req.uri().clone();
    let headers = req.headers().clone();

    // Get the body once and clone it for retries
    let body_bytes = req.collect().await?.to_bytes();

    for attempt in 1..=MAX_RETRIES {
        // Build a new request for each attempt
        let mut upstream_req = Request::builder().method(&method);

        // Build the upstream request URL using the target service port
        let upstream_url = format!(
            "http://127.0.0.1:{}{}",
            target_service.port(),
            uri.path_and_query().map(|x| x.as_str()).unwrap_or("/")
        );

        upstream_req = upstream_req.uri(&upstream_url);

        // Copy headers (except host)
        for (name, value) in &headers {
            if name != "host" {
                upstream_req = upstream_req.header(name, value);
            }
        }

        let upstream_req = upstream_req.body(Full::new(body_bytes.clone()))?;

        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();

        match timeout(Duration::from_secs(10), client.request(upstream_req)).await {
            Ok(Ok(upstream_resp)) => {
                info!(
                    "✅ Request to {} succeeded on attempt {}",
                    target_service.name(),
                    attempt
                );

                // Build response
                let mut resp_builder = Response::builder().status(upstream_resp.status());

                // Copy response headers and add CORS
                for (name, value) in upstream_resp.headers() {
                    resp_builder = resp_builder.header(name, value);
                }
                resp_builder = resp_builder.header("Access-Control-Allow-Origin", "*");

                // Get response body
                let response_body_bytes = upstream_resp.collect().await?.to_bytes();

                return Ok(resp_builder.body(full_body(response_body_bytes))?);
            }
            Ok(Err(err)) => {
                warn!(
                    "⚠️ Request to {} failed on attempt {}/{}: {}",
                    target_service.name(),
                    attempt,
                    MAX_RETRIES,
                    err
                );
            }
            Err(_) => {
                warn!(
                    "⏰ Request to {} timed out on attempt {}/{}",
                    target_service.name(),
                    attempt,
                    MAX_RETRIES
                );
            }
        }

        // Wait before retrying (except on last attempt)
        if attempt < MAX_RETRIES {
            sleep(Duration::from_millis(RETRY_DELAY_MS * attempt as u64)).await;
        }
    }

    Err(format!(
        "All {} retry attempts failed for {}",
        MAX_RETRIES,
        target_service.name()
    )
    .into())
}

async fn proxy_request(
    req: Request<Incoming>,
    target_service: TargetService,
) -> Result<Response<BoxBody>, Box<dyn std::error::Error + Send + Sync>> {
    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build_http();

    // Build the upstream request URL using the target service port
    let upstream_url = format!(
        "http://127.0.0.1:{}{}",
        target_service.port(),
        req.uri()
            .path_and_query()
            .map(|x| x.as_str())
            .unwrap_or("/")
    );

    info!(
        "Routing request to {} at {}",
        target_service.name(),
        upstream_url
    );

    // Build upstream request
    let mut upstream_req = Request::builder().method(req.method()).uri(upstream_url);

    // Copy headers (except host)
    for (name, value) in req.headers() {
        if name != "host" {
            upstream_req = upstream_req.header(name, value);
        }
    }

    // Get the body
    let body_bytes = req.collect().await?.to_bytes();
    let upstream_req = upstream_req.body(Full::new(body_bytes.clone()))?;

    // Make the request
    let upstream_resp = client.request(upstream_req).await?;

    // Build response
    let mut resp_builder = Response::builder().status(upstream_resp.status());

    // Copy response headers and add CORS
    for (name, value) in upstream_resp.headers() {
        resp_builder = resp_builder.header(name, value);
    }
    resp_builder = resp_builder.header("Access-Control-Allow-Origin", "*");

    // Get response body
    let body_bytes = upstream_resp.collect().await?.to_bytes();

    Ok(resp_builder.body(full_body(body_bytes))?)
}

fn empty_body() -> BoxBody {
    Full::new(Bytes::new())
        .map_err(|never| match never {})
        .boxed()
}

fn full_body<T: Into<Bytes>>(chunk: T) -> BoxBody {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

#[derive(Debug, Clone)]
enum TargetService {
    UserService,
    ProductService,
}

impl TargetService {
    fn port(&self) -> u16 {
        match self {
            TargetService::UserService => 8080,
            TargetService::ProductService => 8081,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            TargetService::UserService => "User Service",
            TargetService::ProductService => "Product Service",
        }
    }
}

fn determine_target_service(path: &str) -> TargetService {
    if path.starts_with("/api/users") || path.contains("user") {
        TargetService::UserService
    } else if path.starts_with("/api/products") || path.contains("product") {
        TargetService::ProductService
    } else {
        // Default to user service for backward compatibility
        TargetService::UserService
    }
}

// Global health checker instance
static HEALTH_CHECKER: tokio::sync::OnceCell<Arc<HealthChecker>> =
    tokio::sync::OnceCell::const_new();

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Gateway...");

    let addr = "127.0.0.1:8082";
    let listener = TcpListener::bind(addr).await?;

    // Initialize health checker
    let health_checker = Arc::new(HealthChecker::new());
    HEALTH_CHECKER.set(Arc::clone(&health_checker)).unwrap();

    // Start health checks
    health_checker.start_health_checks().await;

    info!("🌐 Gateway started on http://{}", addr);
    info!("Routing configuration:");
    info!("  - User Service: http://127.0.0.1:8080 (paths: /api/users, *user*)");
    info!("  - Product Service: http://127.0.0.1:8081 (paths: /api/products, *product*)");
    info!("  - Default: User Service (for backward compatibility)");
    info!("🔍 Health checks enabled - services will be monitored every 30 seconds");

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(handle_request))
                .await
            {
                error!("Error serving connection: {:?}", err);
            }
        });
    }
}
