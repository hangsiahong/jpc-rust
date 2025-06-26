use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Incoming, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use tokio::net::TcpListener;
use tracing::{error, info};

type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

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

    match proxy_request(req, target_service).await {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Gateway...");

    let addr = "127.0.0.1:8082";
    let listener = TcpListener::bind(addr).await?;

    info!("🌐 Gateway started on http://{}", addr);
    info!("Routing configuration:");
    info!("  - User Service: http://127.0.0.1:8080 (paths: /api/users, *user*)");
    info!("  - Product Service: http://127.0.0.1:8081 (paths: /api/products, *product*)");
    info!("  - Default: User Service (for backward compatibility)");

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
