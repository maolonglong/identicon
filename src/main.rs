use std::borrow::Cow;
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

use axum::error_handling::HandleErrorLayer;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{BoxError, Router};
use bytes::Bytes;
use faststr::FastStr;
use identicon::utils;
use quick_cache::sync::Cache;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

type AppState = Arc<Cache<FastStr, CacheEntry>>;

#[derive(Debug, Clone)]
struct CacheEntry {
    image: Bytes,
    etag: FastStr,
}

async fn gen_image(
    Path(name): Path<FastStr>,
    headers: HeaderMap,
    State(cache): State<AppState>,
) -> Response {
    if name == "favicon.ico" {
        return not_found().await.into_response();
    }

    let entry = cache
        .get_or_insert_async(&name, async {
            let image = identicon::gen(name.as_bytes());

            let mut buf = Vec::with_capacity(3072);
            image
                .write_to(&mut Cursor::new(&mut buf), image::ImageOutputFormat::Png)
                .unwrap();

            let hash = utils::md5(&buf);

            Ok::<_, ()>(CacheEntry {
                image: buf.into(),
                etag: hex::encode(hash).into(),
            })
        })
        .await
        .unwrap();

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

async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "nothing to see here")
}

async fn handle_error(error: BoxError) -> impl IntoResponse {
    if error.is::<tower::timeout::error::Elapsed>() {
        return (StatusCode::REQUEST_TIMEOUT, Cow::from("request timed out"));
    }

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Cow::from(format!("unhandled internal error: {}", error)),
    )
}

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let cache = Cache::new(1024);

    let router = Router::new()
        .route("/:name", get(gen_image))
        .fallback(not_found)
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(handle_error))
                .timeout(Duration::from_secs(10))
                .layer(TraceLayer::new_for_http()),
        )
        .with_state(Arc::new(cache));

    Ok(router.into())
}
