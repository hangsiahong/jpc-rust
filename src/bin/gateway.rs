use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Incoming, Request, Response, Method, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tracing::{info, error};
use std::convert::Infallible;
use http_body_util::{Full, BodyExt};
use bytes::Bytes;

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

    // Proxy to user service
    match proxy_request(req).await {
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
) -> Result<Response<BoxBody>, Box<dyn std::error::Error + Send + Sync>> {
    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build_http();

    // Build the upstream request URL
    let upstream_url = format!("http://127.0.0.1:8080{}", req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("/"));
    
    // Build upstream request
    let mut upstream_req = Request::builder()
        .method(req.method())
        .uri(upstream_url);

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Gateway...");

    let addr = "127.0.0.1:8081";
    let listener = TcpListener::bind(addr).await?;
    
    info!("🌐 Gateway started on http://{}", addr);
    info!("Proxying requests to User Service at http://127.0.0.1:8080");

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
