//! Library surface for the TeamSpace server.
//!
//! Process startup lives in `main.rs`; this module wires the HTTP API and
//! exposes the modules that integration tests exercise against PostgreSQL.

use axum::{
    Router, extract::State, http::StatusCode, routing::delete, routing::get, routing::post,
};
use sqlx::PgPool;

pub mod auth;
pub mod errors;
pub mod members;
pub mod password;
pub mod projects;
pub mod sessions;
pub mod users;

/// Builds the HTTP application with all routes and shared state.
pub fn app(pool: PgPool) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/health/db", get(health_db))
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/me", get(auth::me))
        .route(
            "/projects",
            get(projects::list_projects).post(projects::create_project),
        )
        .route(
            "/projects/{project_id}",
            get(projects::get_project)
                .patch(projects::update_project)
                .delete(projects::delete_project),
        )
        .route(
            "/projects/{project_id}/members",
            get(members::list_members).post(members::add_member),
        )
        .route(
            "/projects/{project_id}/members/{user_id}",
            delete(members::remove_member),
        )
        .with_state(pool)
}

async fn health() -> &'static str {
    "TeamSpace API is running!"
}

async fn health_db(State(pool): State<PgPool>) -> Result<&'static str, (StatusCode, &'static str)> {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&pool)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database connection failed",
            )
        })?;

    Ok("TeamSpace database connected!")
}
