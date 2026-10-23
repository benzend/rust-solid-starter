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
            Self::BadJson(rejection) => (rejection.status(), rejection.body_text()),
            // Log the details, but never send database internals to the client.
            Self::Database(err) => {
                tracing::error!(error = %err, "database error");
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
