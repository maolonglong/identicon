use std::borrow::Cow;
use std::io::Cursor;
use std::net::SocketAddr;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Duration;

use axum::error_handling::HandleErrorLayer;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{BoxError, Router};
use bytes::Bytes;
use clap::Parser;
use faststr::FastStr;
use humantime::parse_duration;
use identicon::utils;
use lru::LruCache;
use tokio::signal;
use tokio::sync::Mutex;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::{debug, info, instrument};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Listen address
    #[arg(long, default_value = "127.0.0.1:8080")]
    addr: SocketAddr,

    /// Limit the max number of in-flight requests
    #[arg(long = "concurrency", default_value_t = 1024)]
    concurrency_limit: usize,

    /// Request timeout
    #[arg(long, value_parser = parse_duration, default_value = "10s")]
    timeout: Duration,

    /// LRU cache capacity
    #[arg(long, default_value = "64")]
    lru_cap: NonZeroUsize,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    image: Bytes,
    etag: FastStr,
}

type AppState = Arc<Mutex<LruCache<FastStr, CacheEntry>>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let cache = LruCache::new(args.lru_cap);

    let app = Router::new()
        .route("/:name", get(gen_image))
        .fallback(not_found)
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(handle_error))
                .load_shed()
                .concurrency_limit(args.concurrency_limit)
                .timeout(args.timeout)
                .layer(TraceLayer::new_for_http()),
        )
        .with_state(Arc::new(Mutex::new(cache)));

    info!("listening on {}", args.addr);
    axum::Server::bind(&args.addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn gen_image(
    Path(name): Path<FastStr>,
    headers: HeaderMap,
    State(cache): State<AppState>,
) -> Response {
    if name == "favicon.ico" {
        return not_found().await.into_response();
    }

    let entry = {
        let mut guard = cache.lock().await;
        guard.get_or_insert(name.clone(), || load(name)).clone()
    };

    if let Some(etag) = headers.get(header::IF_NONE_MATCH) {
        if let Ok(etag) = etag.to_str() {
            if etag == entry.etag {
                return StatusCode::NOT_MODIFIED.into_response();
            }
        }
    }

    (
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "public, max-age=30672000"),
            (header::ETAG, &entry.etag),
        ],
        entry.image,
    )
        .into_response()
}

#[instrument(skip_all)]
fn load(name: FastStr) -> CacheEntry {
    debug!("cache missing");

    let image = identicon::make(name.as_bytes());

    let mut buf = Vec::with_capacity(3072);
    image
        .write_to(&mut Cursor::new(&mut buf), image::ImageOutputFormat::Png)
        .unwrap();

    let hash = utils::md5(&buf);

    CacheEntry {
        image: buf.into(),
        etag: hex::encode(hash).into(),
    }
}
async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "nothing to see here")
}

async fn handle_error(error: BoxError) -> impl IntoResponse {
    if error.is::<tower::timeout::error::Elapsed>() {
        return (StatusCode::REQUEST_TIMEOUT, Cow::from("request timed out"));
    }

    if error.is::<tower::load_shed::error::Overloaded>() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Cow::from("service is overloaded, try again later"),
        );
    }

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Cow::from(format!("unhandled internal error: {}", error)),
    )
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
