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
    auth::CurrentUser,
    error::{AppError, AppJson, AppResult},
};

// ---------- Shared types ----------
// `#[ts(export)]` writes a matching .ts file to web/src/types (run `make types`).
// Rename or retype a field here and the frontend stops compiling until it's fixed.

#[derive(Serialize, TS)]
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
// Every route requires a logged-in user (`CurrentUser`), and every query is
// scoped to that user, so nobody can see or change another user's todos.

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/todos", get(list).post(create))
        .route("/todos/{id}", patch(update).delete(remove))
}

async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<Vec<Todo>>> {
    // `query_as!` checks the SQL, column names and types against the schema at
    // compile time (via the .sqlx/ cache; see `make prepare`).
    let todos = sqlx::query_as!(
        Todo,
        "SELECT id, title, done FROM todos WHERE user_id = $1 ORDER BY id",
        user.id
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(todos))
}

async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    AppJson(input): AppJson<NewTodo>,
) -> AppResult<(StatusCode, Json<Todo>)> {
    let title = validate_title(&input.title)?;
    // Parameters are always bound ($1), never formatted into the SQL string.
    let todo = sqlx::query_as!(
        Todo,
        "INSERT INTO todos (title, user_id) VALUES ($1, $2) RETURNING id, title, done",
        title,
        user.id
    )
    .fetch_one(&state.db)
    .await?;
    Ok((StatusCode::CREATED, Json(todo)))
}

async fn update(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i32>,
    AppJson(input): AppJson<UpdateTodo>,
) -> AppResult<Json<Todo>> {
    sqlx::query_as!(
        Todo,
        "UPDATE todos SET done = $1 WHERE id = $2 AND user_id = $3 RETURNING id, title, done",
        input.done,
        id,
        user.id
    )
    .fetch_optional(&state.db)
    .await?
    .map(Json)
    .ok_or(AppError::NotFound)
}

async fn remove(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i32>,
) -> AppResult<StatusCode> {
    let result = sqlx::query!(
        "DELETE FROM todos WHERE id = $1 AND user_id = $2",
        id,
        user.id
    )
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
