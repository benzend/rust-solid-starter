#![allow(clippy::unwrap_used)]

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use server::{
    AppState, app,
    mailer::{Email, Mailer},
};
use sqlx::PgPool;
use tower::ServiceExt;

// Each #[sqlx::test] gets its own fresh database with migrations applied,
// so tests can run in parallel without stepping on each other.

const PASSWORD: &str = "correct horse battery";

struct TestApp {
    router: Router,
    outbox: Arc<Mutex<Vec<Email>>>,
}

fn test_app(db: PgPool) -> TestApp {
    let outbox = Arc::new(Mutex::new(Vec::new()));
    let state = AppState::new(db, "http://localhost:5173", Mailer::Memory(outbox.clone()));
    TestApp {
        router: app(state, "../web/dist", Duration::from_secs(10)),
        outbox,
    }
}

/// A browser-like client: keeps the session cookie between requests.
struct Client<'a> {
    app: &'a TestApp,
    cookie: Option<String>,
}

struct Res {
    status: StatusCode,
    body: Value,
    set_cookie: Option<String>,
}

impl<'a> Client<'a> {
    fn new(app: &'a TestApp) -> Self {
        Self { app, cookie: None }
    }

    /// A client that has signed up (and so is logged in) as `email`.
    async fn signed_up(app: &'a TestApp, email: &str) -> Self {
        let mut client = Self::new(app);
        let res = client
            .send(
                "POST",
                "/api/auth/signup",
                Some(json!({ "email": email, "password": PASSWORD })),
            )
            .await;
        assert_eq!(res.status, StatusCode::CREATED);
        client
    }

    async fn send(&mut self, method: &str, uri: &str, body: Option<Value>) -> Res {
        self.send_raw(method, uri, body.map(|b| b.to_string()), &[])
            .await
    }

    async fn send_raw(
        &mut self,
        method: &str,
        uri: &str,
        body: Option<String>,
        headers: &[(&str, &str)],
    ) -> Res {
        let mut req = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json");
        if let Some(cookie) = &self.cookie {
            req = req.header(header::COOKIE, cookie);
        }
        for (name, value) in headers {
            req = req.header(*name, *value);
        }
        let req = req.body(body.map_or_else(Body::empty, Body::from)).unwrap();
        let res = self.app.router.clone().oneshot(req).await.unwrap();

        let status = res.status();
        let set_cookie = res
            .headers()
            .get(header::SET_COOKIE)
            .map(|v| v.to_str().unwrap().to_owned());
        if let Some(set) = &set_cookie {
            let pair = set.split(';').next().unwrap();
            self.cookie = (!pair.ends_with('=')).then(|| pair.to_owned());
        }
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        Res {
            status,
            body,
            set_cookie,
        }
    }
}

// ---------- Todos ----------

#[sqlx::test]
async fn creates_updates_and_deletes_a_todo(db: PgPool) {
    let app = test_app(db);
    let mut client = Client::signed_up(&app, "ada@example.com").await;

    let res = client
        .send(
            "POST",
            "/api/todos",
            Some(json!({ "title": "  Write tests " })),
        )
        .await;
    assert_eq!(res.status, StatusCode::CREATED);
    assert_eq!(res.body["title"], "Write tests");
    assert_eq!(res.body["done"], false);
    let id = res.body["id"].as_i64().unwrap();

    let res = client
        .send(
            "PATCH",
            &format!("/api/todos/{id}"),
            Some(json!({ "done": true })),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.body["done"], true);

    let res = client
        .send("DELETE", &format!("/api/todos/{id}"), None)
        .await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);

    let res = client.send("GET", "/api/todos", None).await;
    assert_eq!(res.body, json!([]));
}

#[sqlx::test]
async fn todos_require_login(db: PgPool) {
    let app = test_app(db);
    let res = Client::new(&app).send("GET", "/api/todos", None).await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
    assert_eq!(res.body["message"], "Please log in.");
}

#[sqlx::test]
async fn users_only_see_their_own_todos(db: PgPool) {
    let app = test_app(db);
    let mut ada = Client::signed_up(&app, "ada@example.com").await;
    let mut bob = Client::signed_up(&app, "bob@example.com").await;

    let res = ada
        .send("POST", "/api/todos", Some(json!({ "title": "Ada's" })))
        .await;
    let id = res.body["id"].as_i64().unwrap();

    assert_eq!(bob.send("GET", "/api/todos", None).await.body, json!([]));
    let res = bob
        .send(
            "PATCH",
            &format!("/api/todos/{id}"),
            Some(json!({ "done": true })),
        )
        .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    let res = bob.send("DELETE", &format!("/api/todos/{id}"), None).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn rejects_blank_titles(db: PgPool) {
    let app = test_app(db);
    let mut client = Client::signed_up(&app, "ada@example.com").await;
    let res = client
        .send("POST", "/api/todos", Some(json!({ "title": "   " })))
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(res.body["message"], "Title must be 1-200 characters.");
}

#[sqlx::test]
async fn missing_field_uses_the_error_shape(db: PgPool) {
    let app = test_app(db);
    let mut client = Client::signed_up(&app, "ada@example.com").await;
    let res = client
        .send("POST", "/api/todos", Some(json!({ "nope": 1 })))
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(res.body["message"].as_str().unwrap().contains("title"));
}

#[sqlx::test]
async fn malformed_json_uses_the_error_shape(db: PgPool) {
    let app = test_app(db);
    let mut client = Client::signed_up(&app, "ada@example.com").await;
    let res = client
        .send_raw("POST", "/api/todos", Some(r#"{"title": "#.to_owned()), &[])
        .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert!(res.body["message"].as_str().unwrap().contains("JSON"));
}

#[sqlx::test]
async fn missing_todo_is_404(db: PgPool) {
    let app = test_app(db);
    let mut client = Client::signed_up(&app, "ada@example.com").await;
    let res = client
        .send("PATCH", "/api/todos/999", Some(json!({ "done": true })))
        .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    assert_eq!(res.body["message"], "Not found.");
}

#[sqlx::test]
async fn unknown_api_route_is_json_404(db: PgPool) {
    let app = test_app(db);
    let res = Client::new(&app).send("GET", "/api/nope", None).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    assert_eq!(res.body["message"], "Not found.");
}

// ---------- Auth ----------

#[sqlx::test]
async fn signs_up_logs_out_and_logs_back_in(db: PgPool) {
    let app = test_app(db);
    let mut client = Client::new(&app);

    let res = client
        .send(
            "POST",
            "/api/auth/signup",
            Some(json!({ "email": " Ada@Example.com ", "password": PASSWORD })),
        )
        .await;
    assert_eq!(res.status, StatusCode::CREATED);
    assert_eq!(res.body["email"], "ada@example.com");
    let cookie = res.set_cookie.unwrap();
    assert!(cookie.contains("HttpOnly") && cookie.contains("SameSite=Lax"));

    let res = client.send("GET", "/api/auth/me", None).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.body["email"], "ada@example.com");

    let old_cookie = client.cookie.clone();
    let res = client.send("POST", "/api/auth/logout", None).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert!(res.set_cookie.unwrap().contains("Max-Age=0"));
    // The old token no longer works, even if a copy of it was kept.
    client.cookie = old_cookie;
    assert_eq!(
        client.send("GET", "/api/auth/me", None).await.status,
        StatusCode::UNAUTHORIZED
    );

    let res = client
        .send(
            "POST",
            "/api/auth/login",
            Some(json!({ "email": "ADA@example.com", "password": PASSWORD })),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(
        client.send("GET", "/api/todos", None).await.status,
        StatusCode::OK
    );
}

#[sqlx::test]
async fn rejects_wrong_passwords_and_unknown_emails_alike(db: PgPool) {
    let app = test_app(db);
    Client::signed_up(&app, "ada@example.com").await;

    for email in ["ada@example.com", "nobody@example.com"] {
        let res = Client::new(&app)
            .send(
                "POST",
                "/api/auth/login",
                Some(json!({ "email": email, "password": "wrong password" })),
            )
            .await;
        assert_eq!(res.status, StatusCode::UNAUTHORIZED);
        assert_eq!(res.body["message"], "Email or password is incorrect.");
        assert!(res.set_cookie.is_none());
    }
}

#[sqlx::test]
async fn validates_sign_ups(db: PgPool) {
    let app = test_app(db);
    Client::signed_up(&app, "ada@example.com").await;

    let cases = [
        (
            json!({ "email": "not-an-email", "password": PASSWORD }),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            json!({ "email": "bob@example.com", "password": "short" }),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            json!({ "email": "ADA@example.com", "password": PASSWORD }),
            StatusCode::CONFLICT,
        ),
    ];
    for (body, status) in cases {
        let res = Client::new(&app)
            .send("POST", "/api/auth/signup", Some(body))
            .await;
        assert_eq!(res.status, status);
    }
}

#[sqlx::test]
async fn rate_limits_login_attempts_per_account(db: PgPool) {
    let app = test_app(db);
    let attempt = json!({ "email": "ada@example.com", "password": "wrong password" });
    for _ in 0..10 {
        let res = Client::new(&app)
            .send("POST", "/api/auth/login", Some(attempt.clone()))
            .await;
        assert_eq!(res.status, StatusCode::UNAUTHORIZED);
    }
    let res = Client::new(&app)
        .send("POST", "/api/auth/login", Some(attempt))
        .await;
    assert_eq!(res.status, StatusCode::TOO_MANY_REQUESTS);
}

#[sqlx::test]
async fn resets_a_forgotten_password(db: PgPool) {
    let app = test_app(db);
    let mut old_session = Client::signed_up(&app, "ada@example.com").await;

    let res = Client::new(&app)
        .send(
            "POST",
            "/api/auth/forgot-password",
            Some(json!({ "email": "ada@example.com" })),
        )
        .await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    // Unknown emails get the same answer and no email.
    let res = Client::new(&app)
        .send(
            "POST",
            "/api/auth/forgot-password",
            Some(json!({ "email": "nobody@example.com" })),
        )
        .await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);

    // The email is sent in the background.
    tokio::time::sleep(Duration::from_millis(50)).await;
    let email = {
        let outbox = app.outbox.lock().unwrap();
        assert_eq!(outbox.len(), 1);
        outbox[0].clone()
    };
    assert_eq!(email.to, "ada@example.com");
    let token = email
        .text
        .split("/reset-password#token=")
        .nth(1)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_owned();

    let mut client = Client::new(&app);
    let reset = json!({ "token": token, "password": "a brand new password" });
    let res = client
        .send("POST", "/api/auth/reset-password", Some(reset.clone()))
        .await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(
        client.send("GET", "/api/auth/me", None).await.status,
        StatusCode::OK
    );

    // Sessions from before the reset are logged out, and the link is single-use.
    assert_eq!(
        old_session.send("GET", "/api/auth/me", None).await.status,
        StatusCode::UNAUTHORIZED
    );
    let res = Client::new(&app)
        .send("POST", "/api/auth/reset-password", Some(reset))
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);

    let res = Client::new(&app)
        .send(
            "POST",
            "/api/auth/login",
            Some(json!({ "email": "ada@example.com", "password": "a brand new password" })),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK);
}

#[sqlx::test]
async fn blocks_cross_site_requests_that_change_data(db: PgPool) {
    let app = test_app(db);
    let mut client = Client::signed_up(&app, "ada@example.com").await;
    let body = Some(json!({ "title": "Forged" }).to_string());

    let res = client
        .send_raw(
            "POST",
            "/api/todos",
            body.clone(),
            &[("sec-fetch-site", "cross-site")],
        )
        .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
    let res = client
        .send_raw(
            "POST",
            "/api/todos",
            body.clone(),
            &[
                ("origin", "https://evil.example"),
                ("host", "app.example.com"),
            ],
        )
        .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);

    let res = client
        .send_raw(
            "POST",
            "/api/todos",
            body.clone(),
            &[("sec-fetch-site", "same-origin")],
        )
        .await;
    assert_eq!(res.status, StatusCode::CREATED);
    let res = client
        .send_raw(
            "POST",
            "/api/todos",
            body,
            &[
                ("origin", "https://app.example.com"),
                ("host", "app.example.com"),
            ],
        )
        .await;
    assert_eq!(res.status, StatusCode::CREATED);
    // Reads aren't blocked.
    let res = client
        .send_raw(
            "GET",
            "/api/todos",
            None,
            &[("sec-fetch-site", "cross-site")],
        )
        .await;
    assert_eq!(res.status, StatusCode::OK);
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
