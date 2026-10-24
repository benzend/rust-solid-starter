#![allow(clippy::unwrap_used)]

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use server::{AppState, app};
use sqlx::PgPool;
use std::time::Duration;
use tower::ServiceExt;

// Each #[sqlx::test] gets its own fresh database with migrations applied,
// so tests can run in parallel without stepping on each other.

fn test_app(db: PgPool) -> Router {
    app(AppState { db }, "../web/dist", Duration::from_secs(10))
}

async fn send(app: &Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    send_raw(app, method, uri, body.map(|b| b.to_string())).await
}

async fn send_raw(
    app: &Router,
    method: &str,
    uri: &str,
    body: Option<String>,
) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(body.map_or_else(Body::empty, Body::from))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

#[sqlx::test]
async fn creates_updates_and_deletes_a_todo(db: PgPool) {
    let app = test_app(db);

    let (status, todo) = send(
        &app,
        "POST",
        "/api/todos",
        Some(json!({ "title": "  Write tests " })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(todo["title"], "Write tests");
    assert_eq!(todo["done"], false);
    let id = todo["id"].as_i64().unwrap();

    let (status, todo) = send(
        &app,
        "PATCH",
        &format!("/api/todos/{id}"),
        Some(json!({ "done": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(todo["done"], true);

    let (status, _) = send(&app, "DELETE", &format!("/api/todos/{id}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, list) = send(&app, "GET", "/api/todos", None).await;
    assert_eq!(list, json!([]));
}

#[sqlx::test]
async fn rejects_blank_titles(db: PgPool) {
    let (status, body) = send(
        &test_app(db),
        "POST",
        "/api/todos",
        Some(json!({ "title": "   " })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["message"], "Title must be 1-200 characters.");
}

#[sqlx::test]
async fn missing_field_uses_the_error_shape(db: PgPool) {
    let (status, body) = send(
        &test_app(db),
        "POST",
        "/api/todos",
        Some(json!({ "nope": 1 })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body["message"].as_str().unwrap().contains("title"));
}

#[sqlx::test]
async fn malformed_json_uses_the_error_shape(db: PgPool) {
    let (status, body) = send_raw(
        &test_app(db),
        "POST",
        "/api/todos",
        Some(r#"{"title": "#.to_owned()),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["message"].as_str().unwrap().contains("JSON"));
}

#[sqlx::test]
async fn missing_todo_is_404(db: PgPool) {
    let (status, body) = send(
        &test_app(db),
        "PATCH",
        "/api/todos/999",
        Some(json!({ "done": true })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], "Not found.");
}

#[sqlx::test]
async fn unknown_api_route_is_json_404(db: PgPool) {
    let (status, body) = send(&test_app(db), "GET", "/api/nope", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], "Not found.");
}

// ---------- Middleware (no database needed) ----------

fn panicking_app() -> Router {
    let routes = Router::new().route(
        "/boom",
        axum::routing::get(|| async {
            panic!("kaboom");
            #[allow(unreachable_code)]
            ""
        }),
    );
    server::middleware(routes, Duration::from_secs(10))
}

#[tokio::test]
async fn panics_become_a_json_500() {
    let res = panicking_app()
        .oneshot(Request::get("/boom").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(res.headers().contains_key("x-request-id"));
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["message"], "Something went wrong.");
}

#[tokio::test]
async fn echoes_a_client_request_id() {
    let res = panicking_app()
        .oneshot(
            Request::get("/nope")
                .header("x-request-id", "abc-123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.headers()["x-request-id"], "abc-123");
}

#[tokio::test]
async fn sets_security_headers() {
    let res = panicking_app()
        .oneshot(Request::get("/nope").body(Body::empty()).unwrap())
        .await
        .unwrap();
    for name in [
        "content-security-policy",
        "strict-transport-security",
        "x-content-type-options",
        "x-frame-options",
        "referrer-policy",
    ] {
        assert!(res.headers().contains_key(name), "missing {name}");
    }
}
