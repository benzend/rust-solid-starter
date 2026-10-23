use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch},
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    AppState,
    error::{AppError, AppJson, AppResult},
};

// ---------- Shared types ----------
// `#[ts(export)]` writes a matching .ts file to web/src/types (run `make types`).
// Rename or retype a field here and the frontend stops compiling until it's fixed.

#[derive(Serialize, sqlx::FromRow, TS)]
#[ts(export)]
pub struct Todo {
    pub id: i32,
    pub title: String,
    pub done: bool,
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct NewTodo {
    pub title: String,
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct UpdateTodo {
    pub done: bool,
}

// ---------- Routes ----------

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/todos", get(list).post(create))
        .route("/todos/{id}", patch(update).delete(remove))
}

async fn list(State(state): State<AppState>) -> AppResult<Json<Vec<Todo>>> {
    let todos = sqlx::query_as::<_, Todo>("SELECT id, title, done FROM todos ORDER BY id")
        .fetch_all(&state.db)
        .await?;
    Ok(Json(todos))
}

async fn create(
    State(state): State<AppState>,
    AppJson(input): AppJson<NewTodo>,
) -> AppResult<(StatusCode, Json<Todo>)> {
    let title = validate_title(&input.title)?;
    // Parameters are always bound ($1), never formatted into the SQL string.
    let todo = sqlx::query_as::<_, Todo>(
        "INSERT INTO todos (title) VALUES ($1) RETURNING id, title, done",
    )
    .bind(title)
    .fetch_one(&state.db)
    .await?;
    Ok((StatusCode::CREATED, Json(todo)))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    AppJson(input): AppJson<UpdateTodo>,
) -> AppResult<Json<Todo>> {
    sqlx::query_as::<_, Todo>("UPDATE todos SET done = $1 WHERE id = $2 RETURNING id, title, done")
        .bind(input.done)
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}

async fn remove(State(state): State<AppState>, Path(id): Path<i32>) -> AppResult<StatusCode> {
    let result = sqlx::query("DELETE FROM todos WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

// Validate on the server even though the UI also checks: never trust the client.
fn validate_title(raw: &str) -> AppResult<&str> {
    let title = raw.trim();
    if title.is_empty() || title.chars().count() > 200 {
        return Err(AppError::Validation(
            "Title must be 1-200 characters.".into(),
        ));
    }
    Ok(title)
}
