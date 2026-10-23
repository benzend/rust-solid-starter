pub mod error;
pub mod reporting;
pub mod todos;

use std::{any::Any, time::Duration};

use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{HeaderName, HeaderValue, Request, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use sqlx::PgPool;
use tower_http::{
    catch_panic::CatchPanicLayer,
    compression::CompressionLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
    timeout::TimeoutLayer,
    trace::{DefaultOnFailure, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

use crate::error::{ApiError, AppError, AppResult};

/// Header carrying each request's ID. Clients can send their own; otherwise a
/// UUID is generated. It's on every log line for the request and echoed back
/// in the response, so a user's error report can be matched to the server logs.
pub const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

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

    middleware(
        Router::new().nest("/api", api).fallback_service(spa),
        request_timeout,
    )
}

/// The middleware stack every request goes through. Public so tests can wrap
/// their own routes in it.
pub fn middleware(router: Router, request_timeout: Duration) -> Router {
    // Conservative browser security headers. The CSP only allows scripts, styles
    // and requests from this same origin, which is all a Vite build needs.
    let set = |name: HeaderName, value: &'static str| {
        SetResponseHeaderLayer::if_not_present(name, HeaderValue::from_static(value))
    };

    router
        // A panicking handler becomes a logged 500 with the usual error body,
        // instead of a dropped connection.
        .layer(CatchPanicLayer::custom(panic_response))
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
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(request_span)
                // One line per request (status, latency) as an access log.
                .on_response(DefaultOnResponse::new().level(Level::INFO))
                // Failed responses are logged as warnings; errors worth reporting
                // are logged at ERROR where they happen (database, panics).
                .on_failure(DefaultOnFailure::new().level(Level::WARN)),
        )
        // Outermost: give the request an ID first, and copy it to the response.
        .layer(PropagateRequestIdLayer::new(REQUEST_ID_HEADER))
        .layer(SetRequestIdLayer::new(REQUEST_ID_HEADER, MakeRequestUuid))
}

/// The span every request's logs are recorded in, including its request ID.
fn request_span(req: &Request<Body>) -> tracing::Span {
    let request_id = req
        .headers()
        .get(&REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    tracing::info_span!("request", method = %req.method(), uri = %req.uri(), request_id)
}

fn panic_response(panic: Box<dyn Any + Send + 'static>) -> Response {
    let detail = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
        .unwrap_or("unknown panic");
    tracing::error!(panic = detail, "handler panicked");
    let body = ApiError {
        message: "Something went wrong.".to_owned(),
    };
    (StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response()
}

async fn health(State(state): State<AppState>) -> AppResult<&'static str> {
    sqlx::query!("SELECT 1 AS ok").fetch_one(&state.db).await?;
    Ok("ok")
}
