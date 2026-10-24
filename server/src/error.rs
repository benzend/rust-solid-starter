use axum::{
    Json,
    extract::{FromRequest, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use ts_rs::TS;

/// The JSON body of every error response.
#[derive(Serialize, TS)]
#[ts(export)]
pub struct ApiError {
    pub message: String,
}

/// One error type for every handler. Return `AppResult<T>` and use `?`.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Validation(String),
    #[error("not found")]
    NotFound,
    #[error("not logged in")]
    Unauthorized,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("cross-origin request")]
    CrossOrigin,
    #[error("{0}")]
    Conflict(String),
    #[error("rate limited")]
    TooManyRequests,
    /// Unexpected failures outside the database. Logged; the client gets a 500.
    #[error("{0}")]
    Internal(String),
    #[error(transparent)]
    BadJson(#[from] JsonRejection),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

pub type AppResult<T> = Result<T, AppError>;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::Validation(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg),
            Self::NotFound => (StatusCode::NOT_FOUND, "Not found.".to_owned()),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "Please log in.".to_owned()),
            Self::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "Email or password is incorrect.".to_owned(),
            ),
            Self::CrossOrigin => (
                StatusCode::FORBIDDEN,
                "Cross-origin request blocked.".to_owned(),
            ),
            Self::Conflict(msg) => (StatusCode::CONFLICT, msg),
            Self::TooManyRequests => (
                StatusCode::TOO_MANY_REQUESTS,
                "Too many attempts. Try again later.".to_owned(),
            ),
            Self::BadJson(rejection) => (rejection.status(), rejection.body_text()),
            // Log the details, but never send database internals to the client.
            Self::Database(err) => {
                tracing::error!(error = %err, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong.".to_owned(),
                )
            }
            Self::Internal(err) => {
                tracing::error!(error = %err, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong.".to_owned(),
                )
            }
        };
        (status, Json(ApiError { message })).into_response()
    }
}

/// Drop-in replacement for `axum::Json` as an extractor, so malformed request
/// bodies get the same `ApiError` shape as every other error.
#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(AppError))]
pub struct AppJson<T>(pub T);
