//! Email + password accounts with server-side sessions.
//!
//! Logging in stores a random token in an HttpOnly cookie and its SHA-256 hash
//! in the `sessions` table. Handlers that need a logged-in user take a
//! `CurrentUser` argument; requests without a valid session get a 401.

use std::{sync::LazyLock, time::Duration};

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use axum::{
    Json, Router,
    extract::{FromRequestParts, State},
    http::{HeaderMap, StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::Instrument;
use ts_rs::TS;

use crate::{
    AppState,
    error::{AppError, AppJson, AppResult},
    mailer::Email,
    rate_limit::client_ip,
};

/// Sessions last this long after the app was last opened (see `me`).
const SESSION_DAYS: i32 = 30;
const RESET_LINK_MINUTES: i32 = 60;
const MINUTE: Duration = Duration::from_secs(60);

// ---------- Shared types ----------

#[derive(Serialize, TS, Clone, Debug)]
#[ts(export)]
pub struct User {
    // i64 would export as `bigint`, but JSON numbers arrive as `number`.
    #[ts(type = "number")]
    pub id: i64,
    pub email: String,
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct Credentials {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct ForgotPassword {
    pub email: String,
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct ResetPassword {
    pub token: String,
    pub password: String,
}

// ---------- Routes ----------

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/signup", post(signup))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
        .route("/auth/forgot-password", post(forgot_password))
        .route("/auth/reset-password", post(reset_password))
}

async fn signup(
    State(state): State<AppState>,
    headers: HeaderMap,
    AppJson(input): AppJson<Credentials>,
) -> AppResult<Response> {
    if let Some(ip) = client_ip(&headers) {
        state
            .limiter
            .check(format!("signup:{ip}"), 10, 60 * MINUTE)?;
    }
    let email = normalize_email(&input.email)?;
    validate_password(&input.password)?;
    let password_hash = hash_password(input.password).await?;

    // Note this tells the caller whether an email has an account. Hiding that
    // would need email verification before sign-up completes.
    let user = sqlx::query_as!(
        User,
        "INSERT INTO users (email, password_hash) VALUES ($1, $2)
         ON CONFLICT (email) DO NOTHING
         RETURNING id, email",
        email,
        password_hash
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::Conflict("An account with this email already exists.".into()))?;

    logged_in(&state, user, StatusCode::CREATED).await
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    AppJson(input): AppJson<Credentials>,
) -> AppResult<Response> {
    let email = input.email.trim().to_lowercase();
    // Per account, against password guessing; per IP, against trying many accounts.
    state
        .limiter
        .check(format!("login:{email}"), 10, 15 * MINUTE)?;
    if let Some(ip) = client_ip(&headers) {
        state
            .limiter
            .check(format!("login-ip:{ip}"), 50, 15 * MINUTE)?;
    }

    let account = sqlx::query!(
        "SELECT id, email, password_hash FROM users WHERE email = $1",
        email
    )
    .fetch_optional(&state.db)
    .await?;

    // Without an account, check against a dummy hash anyway, so the response
    // time doesn't reveal which emails have accounts.
    let hash = account
        .as_ref()
        .map_or_else(|| DUMMY_HASH.clone(), |a| a.password_hash.clone());
    let matches = verify_password(input.password, hash).await?;

    match account {
        Some(a) if matches => {
            let user = User {
                id: a.id,
                email: a.email,
            };
            logged_in(&state, user, StatusCode::OK).await
        }
        _ => Err(AppError::InvalidCredentials),
    }
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Response> {
    if let Some(token) = read_cookie(&headers, cookie_name(&state)) {
        sqlx::query!(
            "DELETE FROM sessions WHERE token_hash = $1",
            token_hash(token)
        )
        .execute(&state.db)
        .await?;
    }
    Ok((
        StatusCode::NO_CONTENT,
        [(header::SET_COOKIE, clear_cookie(&state))],
    )
        .into_response())
}

/// The logged-in user, or 401. The app calls this on load, so it also extends
/// the session: users stay logged in as long as they come back within
/// SESSION_DAYS.
async fn me(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Response> {
    let token = read_cookie(&headers, cookie_name(&state)).ok_or(AppError::Unauthorized)?;
    let user = sqlx::query_as!(
        User,
        r#"UPDATE sessions s SET expires_at = now() + make_interval(days => $2)
           FROM users u
           WHERE s.token_hash = $1 AND s.expires_at > now() AND u.id = s.user_id
           RETURNING u.id AS "id!", u.email AS "email!""#,
        token_hash(token),
        SESSION_DAYS
    )
    .fetch_optional(&state.db)
    .await?;

    match user {
        Some(user) => Ok((
            [(header::SET_COOKIE, session_cookie(&state, token))],
            Json(user),
        )
            .into_response()),
        // Clear a stale cookie so the browser stops sending it.
        None => Ok((
            [(header::SET_COOKIE, clear_cookie(&state))],
            AppError::Unauthorized,
        )
            .into_response()),
    }
}

/// Always 204, whether or not the account exists, so this can't be used to
/// find out which emails have accounts.
async fn forgot_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    AppJson(input): AppJson<ForgotPassword>,
) -> AppResult<StatusCode> {
    let email = input.email.trim().to_lowercase();
    state
        .limiter
        .check(format!("forgot:{email}"), 5, 60 * MINUTE)?;
    if let Some(ip) = client_ip(&headers) {
        state
            .limiter
            .check(format!("forgot-ip:{ip}"), 20, 60 * MINUTE)?;
    }

    let Some(user_id) = sqlx::query_scalar!("SELECT id FROM users WHERE email = $1", email)
        .fetch_optional(&state.db)
        .await?
    else {
        return Ok(StatusCode::NO_CONTENT);
    };

    sqlx::query!("DELETE FROM password_resets WHERE expires_at < now()")
        .execute(&state.db)
        .await?;
    let (token, hash) = new_token();
    sqlx::query!(
        "INSERT INTO password_resets (token_hash, user_id, expires_at)
         VALUES ($1, $2, now() + make_interval(mins => $3))",
        hash,
        user_id,
        RESET_LINK_MINUTES
    )
    .execute(&state.db)
    .await?;

    // The token goes in the URL fragment (#), which browsers never send to the
    // server, so it can't end up in access logs.
    let link = format!(
        "{}/reset-password#token={token}",
        state.app_url.trim_end_matches('/')
    );
    let message = Email {
        to: email,
        subject: "Reset your password".into(),
        text: format!(
            "Someone asked to reset the password for this account. To choose a new one, open:\n\n\
             {link}\n\n\
             The link works once and expires in {RESET_LINK_MINUTES} minutes. \
             If you didn't ask for this, you can ignore this email."
        ),
    };
    // Sent in the background, so the response doesn't take longer when the
    // account exists.
    let mailer = state.mailer.clone();
    tokio::spawn(
        async move {
            if let Err(err) = mailer.send(&message).await {
                tracing::error!(error = %err, "sending password reset email failed");
            }
        }
        .in_current_span(),
    );
    Ok(StatusCode::NO_CONTENT)
}

/// Sets a new password from an emailed link, logs out every other session, and
/// logs this browser in.
async fn reset_password(
    State(state): State<AppState>,
    AppJson(input): AppJson<ResetPassword>,
) -> AppResult<Response> {
    validate_password(&input.password)?;

    let mut tx = state.db.begin().await?;
    // Deleting the token first makes each link single-use.
    let user_id = sqlx::query_scalar!(
        "DELETE FROM password_resets WHERE token_hash = $1 AND expires_at > now()
         RETURNING user_id",
        token_hash(&input.token)
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::Validation("This reset link is invalid or has expired.".into()))?;

    let password_hash = hash_password(input.password).await?;
    let user = sqlx::query_as!(
        User,
        "UPDATE users SET password_hash = $2 WHERE id = $1 RETURNING id, email",
        user_id,
        password_hash
    )
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query!("DELETE FROM sessions WHERE user_id = $1", user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM password_resets WHERE user_id = $1", user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    logged_in(&state, user, StatusCode::OK).await
}

// ---------- The extractor ----------

/// Add `CurrentUser(user): CurrentUser` to a handler's arguments to require a
/// logged-in user. Put it before any body extractor such as `AppJson`.
pub struct CurrentUser(pub User);

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> AppResult<Self> {
        let token =
            read_cookie(&parts.headers, cookie_name(state)).ok_or(AppError::Unauthorized)?;
        let user = sqlx::query_as!(
            User,
            "SELECT u.id, u.email FROM sessions s JOIN users u ON u.id = s.user_id
             WHERE s.token_hash = $1 AND s.expires_at > now()",
            token_hash(token)
        )
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::Unauthorized)?;
        Ok(Self(user))
    }
}

// ---------- Sessions and cookies ----------

async fn logged_in(state: &AppState, user: User, status: StatusCode) -> AppResult<Response> {
    sqlx::query!("DELETE FROM sessions WHERE expires_at < now()")
        .execute(&state.db)
        .await?;
    let (token, hash) = new_token();
    sqlx::query!(
        "INSERT INTO sessions (token_hash, user_id, expires_at)
         VALUES ($1, $2, now() + make_interval(days => $3))",
        hash,
        user.id,
        SESSION_DAYS
    )
    .execute(&state.db)
    .await?;
    Ok((
        status,
        [(header::SET_COOKIE, session_cookie(state, &token))],
        Json(user),
    )
        .into_response())
}

/// A random 256-bit token (hex, for the cookie or link) and its hash (for the
/// database).
fn new_token() -> (String, Vec<u8>) {
    let mut bytes = [0u8; 32];
    rand::fill(&mut bytes);
    let token = hex::encode(bytes);
    let hash = token_hash(&token);
    (token, hash)
}

fn token_hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

/// Cookies are `Secure` when APP_URL is https. The `__Host-` prefix then also
/// stops subdomains from setting or overriding the cookie.
fn secure(state: &AppState) -> bool {
    state.app_url.starts_with("https://")
}

fn cookie_name(state: &AppState) -> &'static str {
    if secure(state) {
        "__Host-session"
    } else {
        "session"
    }
}

/// HttpOnly keeps the token away from JavaScript. SameSite=Lax stops other
/// sites from sending it with their requests, except plain links to this app.
fn session_cookie(state: &AppState, token: &str) -> String {
    let max_age = i64::from(SESSION_DAYS) * 24 * 60 * 60;
    format!(
        "{}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{}",
        cookie_name(state),
        if secure(state) { "; Secure" } else { "" }
    )
}

fn clear_cookie(state: &AppState) -> String {
    format!(
        "{}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{}",
        cookie_name(state),
        if secure(state) { "; Secure" } else { "" }
    )
}

fn read_cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value)
}

// ---------- Passwords ----------

fn normalize_email(raw: &str) -> AppResult<String> {
    let email = raw.trim().to_lowercase();
    let valid = email.len() <= 254
        && email
            .split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'));
    if !valid {
        return Err(AppError::Validation("Enter a valid email address.".into()));
    }
    Ok(email)
}

fn validate_password(password: &str) -> AppResult<()> {
    if !(8..=128).contains(&password.chars().count()) {
        return Err(AppError::Validation(
            "Password must be 8-128 characters.".into(),
        ));
    }
    Ok(())
}

static DUMMY_HASH: LazyLock<String> = LazyLock::new(|| {
    Argon2::default()
        .hash_password(b"not anyone's password")
        .map(|hash: PasswordHash| hash.to_string())
        .unwrap_or_default()
});

/// Argon2id with the crate's defaults (OWASP's recommended minimums). Hashing
/// is deliberately slow, so it runs off the async worker threads.
async fn hash_password(password: String) -> AppResult<String> {
    tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password(password.as_bytes())
            .map(|hash: PasswordHash| hash.to_string())
    })
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
    .map_err(|err| AppError::Internal(err.to_string()))
}

async fn verify_password(password: String, hash: String) -> AppResult<bool> {
    tokio::task::spawn_blocking(move || {
        let hash = PasswordHash::new(&hash).map_err(|err| AppError::Internal(err.to_string()))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok())
    })
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
}
