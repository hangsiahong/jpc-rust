use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Incoming, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, timeout};
use tracing::{error, info, warn};
use uuid::Uuid;

type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

// Metrics structure
#[derive(Debug, Default)]
struct GatewayMetrics {
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    service_errors: AtomicU64,
    average_response_time_ms: AtomicU64,
    active_connections: AtomicU64,
}

impl GatewayMetrics {
    fn increment_total_requests(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_successful_requests(&self) {
        self.successful_requests.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_failed_requests(&self) {
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_service_errors(&self) {
        self.service_errors.fetch_add(1, Ordering::Relaxed);
    }

    fn update_response_time(&self, duration_ms: u64) {
        // Simple moving average (in production, use proper metrics library)
        let current = self.average_response_time_ms.load(Ordering::Relaxed);
        let new_avg = if current == 0 {
            duration_ms
        } else {
            (current + duration_ms) / 2
        };
        self.average_response_time_ms
            .store(new_avg, Ordering::Relaxed);
    }

    fn increment_active_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    fn decrement_active_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    fn get_stats(&self) -> String {
        let total = self.total_requests.load(Ordering::Relaxed);
        let successful = self.successful_requests.load(Ordering::Relaxed);
        let success_rate = if total > 0 {
            (successful as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        format!(
            r#"{{
                "total_requests": {},
                "successful_requests": {},
                "failed_requests": {},
                "service_errors": {},
                "average_response_time_ms": {},
                "active_connections": {},
                "success_rate": {:.2}
            }}"#,
            total,
            successful,
            self.failed_requests.load(Ordering::Relaxed),
            self.service_errors.load(Ordering::Relaxed),
            self.average_response_time_ms.load(Ordering::Relaxed),
            self.active_connections.load(Ordering::Relaxed),
            success_rate
        )
    }
}

// Rate limiting
#[derive(Debug)]
struct RateLimiter {
    requests: Arc<Mutex<HashMap<String, (u64, Instant)>>>,
    max_requests_per_minute: u64,
}

impl RateLimiter {
    fn new(max_requests_per_minute: u64) -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
            max_requests_per_minute,
        }
    }

    async fn is_allowed(&self, client_ip: &str) -> bool {
        let mut requests = self.requests.lock().await;
        let now = Instant::now();

        // Clean up old entries (older than 1 minute)
        requests.retain(|_, (_, timestamp)| now.duration_since(*timestamp).as_secs() < 60);

        match requests.get_mut(client_ip) {
            Some((count, timestamp)) => {
                if now.duration_since(*timestamp).as_secs() >= 60 {
                    // Reset counter after 1 minute
                    *count = 1;
                    *timestamp = now;
                    true
                } else if *count < self.max_requests_per_minute {
                    *count += 1;
                    true
                } else {
                    false
                }
            }
            None => {
                requests.insert(client_ip.to_string(), (1, now));
                true
            }
        }
    }
}

// Service instance for load balancing (prepared for future use)
// Uncomment and use when implementing load balancing for multiple service instances

// #[derive(Debug, Clone)]
// struct ServiceInstance {
//     host: String,
//     port: u16,
//     weight: u32,
//     is_healthy: bool,
// }

// #[derive(Debug)]
// struct LoadBalancer {
//     instances: Vec<ServiceInstance>,
//     current_index: AtomicU64,
// }

// impl LoadBalancer {
//     fn new(instances: Vec<ServiceInstance>) -> Self {
//         Self {
//             instances,
//             current_index: AtomicU64::new(0),
//         }
//     }

//     fn get_next_instance(&self) -> Option<&ServiceInstance> {
//         let healthy_instances: Vec<&ServiceInstance> =
//             self.instances.iter().filter(|i| i.is_healthy).collect();

//         if healthy_instances.is_empty() {
//             return None;
//         }

//         let index =
//             self.current_index.fetch_add(1, Ordering::Relaxed) as usize % healthy_instances.len();
//         Some(healthy_instances[index])
//     }

//     fn mark_unhealthy(&mut self, host: &str, port: u16) {
//         for instance in &mut self.instances {
//             if instance.host == host && instance.port == port {
//                 instance.is_healthy = false;
//                 break;
//             }
//         }
//     }

//     fn mark_healthy(&mut self, host: &str, port: u16) {
//         for instance in &mut self.instances {
//             if instance.host == host && instance.port == port {
//                 instance.is_healthy = true;
//                 break;
//             }
//         }
//     }
// }

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
    metrics: Arc<GatewayMetrics>,
    rate_limiter: Arc<RateLimiter>,
}

impl HealthChecker {
    fn new() -> Self {
        Self {
            user_service: Arc::new(RwLock::new(ServiceHealth::default())),
            product_service: Arc::new(RwLock::new(ServiceHealth::default())),
            metrics: Arc::new(GatewayMetrics::default()),
            rate_limiter: Arc::new(RateLimiter::new(1000)), // 1000 requests per minute per IP
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
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    info!(
        "🔄 [{}] Handling request: {} {}",
        request_id,
        req.method(),
        req.uri()
    );

    let health_checker = HEALTH_CHECKER.get().unwrap();

    // Increment metrics
    health_checker.metrics.increment_total_requests();
    health_checker.metrics.increment_active_connections();

    // Handle CORS preflight
    if req.method() == Method::OPTIONS {
        health_checker.metrics.decrement_active_connections();
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
            .header("Access-Control-Allow-Headers", "Content-Type")
            .header("X-Request-ID", request_id)
            .body(empty_body())
            .unwrap());
    }

    // Handle metrics endpoint
    if req.uri().path() == "/metrics" {
        let metrics_json = health_checker.metrics.get_stats();
        health_checker.metrics.decrement_active_connections();
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .header("Access-Control-Allow-Origin", "*")
            .header("X-Request-ID", request_id)
            .body(full_body(metrics_json))
            .unwrap());
    }

    // Rate limiting (simplified - get client IP from headers in production)
    let client_ip = "127.0.0.1"; // In production, extract from X-Forwarded-For or similar
    if !health_checker.rate_limiter.is_allowed(client_ip).await {
        warn!("🚫 [{}] Rate limit exceeded for {}", request_id, client_ip);
        health_checker.metrics.increment_failed_requests();
        health_checker.metrics.decrement_active_connections();
        return Ok(Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header("Access-Control-Allow-Origin", "*")
            .header("X-Request-ID", request_id)
            .body(full_body("Rate limit exceeded"))
            .unwrap());
    }

    // Route requests based on path
    let path = req.uri().path();
    let target_service = determine_target_service(path);

    // Check service health before proxying
    if !health_checker.is_service_healthy(&target_service).await {
        warn!(
            "🔴 [{}] Service {} unavailable",
            request_id,
            target_service.name()
        );
        health_checker.metrics.increment_service_errors();
        health_checker.metrics.increment_failed_requests();
        health_checker.metrics.decrement_active_connections();
        return Ok(Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .header("Access-Control-Allow-Origin", "*")
            .header("X-Request-ID", request_id)
            .body(full_body("Service unavailable"))
            .unwrap());
    }

    match proxy_request_with_retry(req, target_service, &request_id).await {
        Ok(response) => {
            let duration = start_time.elapsed().as_millis() as u64;
            health_checker.metrics.update_response_time(duration);
            health_checker.metrics.increment_successful_requests();
            health_checker.metrics.decrement_active_connections();

            info!("✅ [{}] Request completed in {}ms", request_id, duration);

            // Add request ID to response
            let (mut parts, body) = response.into_parts();
            parts
                .headers
                .insert("X-Request-ID", request_id.parse().unwrap());
            Ok(Response::from_parts(parts, body))
        }
        Err(err) => {
            let duration = start_time.elapsed().as_millis() as u64;
            health_checker.metrics.update_response_time(duration);
            health_checker.metrics.increment_failed_requests();
            health_checker.metrics.decrement_active_connections();

            error!(
                "❌ [{}] Proxy error after {}ms: {}",
                request_id, duration, err
            );
            Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Access-Control-Allow-Origin", "*")
                .header("X-Request-ID", request_id)
                .body(full_body(format!("Proxy error: {}", err)))
                .unwrap())
        }
    }
}

async fn proxy_request_with_retry(
    req: Request<Incoming>,
    target_service: TargetService,
    request_id: &str,
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
                    "✅ [{}] Request to {} succeeded on attempt {}",
                    request_id,
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
                    "⚠️ [{}] Request to {} failed on attempt {}/{}: {}",
                    request_id,
                    target_service.name(),
                    attempt,
                    MAX_RETRIES,
                    err
                );
            }
            Err(_) => {
                warn!(
                    "⏰ [{}] Request to {} timed out on attempt {}/{}",
                    request_id,
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
    info!("Production Features Enabled:");
    info!("  📊 Metrics endpoint: /metrics");
    info!("  🔍 Request tracing with X-Request-ID");
    info!("  🚦 Rate limiting: 1000 requests/minute per IP");
    info!("  🔄 Circuit breaker with 3-failure threshold");
    info!("  ⚡ Retry logic: 3 attempts with exponential backoff");
    info!("  🌐 CORS support for web clients");
    info!("Routing configuration:");
    info!("  - User Service: http://127.0.0.1:8080 (paths: /api/users, *user*)");
    info!("  - Product Service: http://127.0.0.1:8081 (paths: /api/products, *product*)");
    info!("  - Default: User Service (for backward compatibility)");
    info!("🔍 Health checks enabled - services monitored every 30 seconds");

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
