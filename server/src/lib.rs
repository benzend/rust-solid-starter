pub mod error;
pub mod todos;

use std::time::Duration;

use axum::{
    Router,
    extract::State,
    http::{HeaderName, HeaderValue, StatusCode, header},
    routing::get,
};
use sqlx::PgPool;
use tower_http::{
    compression::CompressionLayer,
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
}

/// Builds the whole app: JSON API under /api, the built Solid app everywhere else.
pub fn app(state: AppState, static_dir: &str, request_timeout: Duration) -> Router {
    let api = Router::new()
        .route("/health", get(health))
        .merge(todos::router())
        // Unknown /api routes get a JSON 404 instead of the HTML app.
        .fallback(|| async { AppError::NotFound })
        .with_state(state);

    // Unknown non-API paths fall back to index.html (with a 200) so client-side
    // routes like /about work when loaded directly or refreshed.
    let spa =
        ServeDir::new(static_dir).fallback(ServeFile::new(format!("{static_dir}/index.html")));

    // Conservative browser security headers. The CSP only allows scripts, styles
    // and requests from this same origin, which is all a Vite build needs.
    let set = |name: HeaderName, value: &'static str| {
        SetResponseHeaderLayer::if_not_present(name, HeaderValue::from_static(value))
    };

    Router::new()
        .nest("/api", api)
        .fallback_service(spa)
        .layer(set(
            header::CONTENT_SECURITY_POLICY,
            "default-src 'self'; img-src 'self' data:; object-src 'none'; \
             base-uri 'self'; form-action 'self'; frame-ancestors 'none'",
        ))
        .layer(set(header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
        .layer(set(
            header::REFERRER_POLICY,
            "strict-origin-when-cross-origin",
        ))
        .layer(set(header::X_FRAME_OPTIONS, "DENY"))
        .layer(CompressionLayer::new())
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            request_timeout,
        ))
        .layer(TraceLayer::new_for_http())
}

async fn health(State(state): State<AppState>) -> AppResult<&'static str> {
    sqlx::query!("SELECT 1 AS ok").fetch_one(&state.db).await?;
    Ok("ok")
}
