//! Shared PostgreSQL and HTTP harness for integration tests.
//!
//! Every test runs in its own throwaway schema, applies the real migrations to
//! it and drops it afterwards, so the `public` schema is never touched. When
//! `DATABASE_URL` is not set the individual tests report a skip.
//!
//! The HTTP helpers (`send`, `register_and_login`, ...) drive the real router
//! in-process, so tests exercise routing, extractors and handlers exactly like
//! a network request would.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{
    HeaderMap, Request, StatusCode,
    header::{AUTHORIZATION, CONTENT_TYPE},
};
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use sqlx::{AssertSqlSafe, Connection, Executor, PgConnection, PgPool};
use tower::ServiceExt;

const MIGRATIONS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../migrations");

/// Password used by the HTTP helpers when registering and logging in users.
pub const TEST_PASSWORD: &str = "correct horse battery staple";

/// Disambiguates schemas created by tests running in parallel in one process.
static SCHEMA_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn database_url() -> Option<String> {
    dotenvy::dotenv().ok();
    std::env::var("DATABASE_URL").ok()
}

/// A throwaway schema so tests never modify application data.
pub struct TestSchema {
    name: String,
    admin: PgConnection,
    pub pool: PgPool,
}

impl TestSchema {
    pub async fn new(url: &str) -> Self {
        let name = format!(
            "teamspace_test_{}_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock is before the UNIX epoch")
                .as_nanos(),
            SCHEMA_COUNTER.fetch_add(1, Ordering::Relaxed)
        );

        let mut admin = PgConnection::connect(url)
            .await
            .expect("connect to PostgreSQL for test schema setup");

        // The schema name is generated above from a process id and a timestamp,
        // so it contains no untrusted input.
        sqlx::raw_sql(AssertSqlSafe(format!("CREATE SCHEMA \"{name}\"")))
            .execute(&mut admin)
            .await
            .expect("create the temporary test schema");

        let search_path = format!("SET search_path TO \"{name}\"");
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .after_connect(move |connection, _metadata| {
                let search_path = search_path.clone();
                Box::pin(async move {
                    connection
                        .execute(sqlx::raw_sql(AssertSqlSafe(search_path)))
                        .await?;
                    Ok(())
                })
            })
            .connect(url)
            .await
            .expect("connect to the temporary test schema");

        Self { name, admin, pool }
    }

    pub async fn cleanup(mut self) {
        self.pool.close().await;
        let _ = sqlx::raw_sql(AssertSqlSafe(format!(
            "DROP SCHEMA IF EXISTS \"{}\" CASCADE",
            self.name
        )))
        .execute(&mut self.admin)
        .await;
    }
}

/// Creates an isolated schema with every migration applied and the app on top.
///
/// Returns `None` when `DATABASE_URL` is unset so callers can report a skip.
pub async fn setup() -> Option<(TestSchema, Router)> {
    let url = database_url()?;
    let schema = TestSchema::new(&url).await;
    apply_all_up_migrations(&schema.pool).await;
    let app = server::app(schema.pool.clone());
    Some((schema, app))
}

/// A decoded JSON response from the application router.
#[derive(Debug)]
pub struct ApiResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: String,
    pub json: Value,
}

/// Sends a JSON request through the router without binding a TCP socket.
pub async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
    bearer: Option<&str>,
) -> ApiResponse {
    let raw_body = body.map(|value| value.to_string());
    send_raw(app, method, uri, raw_body.as_deref(), bearer).await
}

/// Sends a request with an already encoded body, for malformed-JSON tests.
pub async fn send_raw(
    app: &Router,
    method: &str,
    uri: &str,
    body: Option<&str>,
    bearer: Option<&str>,
) -> ApiResponse {
    let mut builder = Request::builder().method(method).uri(uri);

    if body.is_some() {
        builder = builder.header(CONTENT_TYPE, "application/json");
    }
    if let Some(token) = bearer {
        builder = builder.header(AUTHORIZATION, format!("Bearer {token}"));
    }

    let response = app
        .clone()
        .oneshot(
            builder
                .body(Body::from(body.unwrap_or_default().to_owned()))
                .expect("build the test request"),
        )
        .await
        .expect("send the test request");

    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read the test response body");
    let text = String::from_utf8(bytes.to_vec()).expect("response body is UTF-8");
    let json = if text.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&text).expect("test response is JSON")
    };

    ApiResponse {
        status,
        headers,
        body: text,
        json,
    }
}

/// Registers a user through `POST /auth/register` and returns its id.
pub async fn register(app: &Router, email: &str, display_name: &str) -> i64 {
    let response = send(
        app,
        "POST",
        "/auth/register",
        Some(json!({
            "email": email,
            "password": TEST_PASSWORD,
            "display_name": display_name,
        })),
        None,
    )
    .await;

    assert_eq!(response.status, StatusCode::CREATED, "{}", response.body);
    response.json["id"].as_i64().expect("numeric user id")
}

/// Logs a registered user in and returns the bearer token.
pub async fn login_token(app: &Router, email: &str) -> String {
    let response = send(
        app,
        "POST",
        "/auth/login",
        Some(json!({ "email": email, "password": TEST_PASSWORD })),
        None,
    )
    .await;

    assert_eq!(response.status, StatusCode::OK, "{}", response.body);
    response.json["token"]
        .as_str()
        .expect("login token")
        .to_owned()
}

/// Registers a user, logs in and returns `(user_id, bearer_token)`.
pub async fn register_and_login(app: &Router, email: &str, display_name: &str) -> (i64, String) {
    let user_id = register(app, email, display_name).await;
    let token = login_token(app, email).await;
    (user_id, token)
}

/// Creates a project through the API and returns its id.
pub async fn create_project(app: &Router, token: &str, name: &str) -> i64 {
    let response = send(
        app,
        "POST",
        "/projects",
        Some(json!({ "name": name })),
        Some(token),
    )
    .await;

    assert_eq!(response.status, StatusCode::CREATED, "{}", response.body);
    response.json["id"].as_i64().expect("numeric project id")
}

/// Asserts that a database error carries the expected SQLSTATE code.
pub fn assert_sqlstate(error: sqlx::Error, expected: &str) {
    let database_error = error
        .as_database_error()
        .unwrap_or_else(|| panic!("expected a database error, got: {error}"));
    assert_eq!(
        database_error.code().as_deref(),
        Some(expected),
        "unexpected SQLSTATE: {database_error}"
    );
}

pub fn up_migration_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(MIGRATIONS_DIR)
        .unwrap_or_else(|error| panic!("read migrations directory {MIGRATIONS_DIR}: {error}"))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.to_string_lossy().ends_with(".up.sql"))
        .collect();
    files.sort();
    files
}

pub fn down_migration_file(up: &Path) -> PathBuf {
    let file_name = up
        .file_name()
        .expect("migration file has a name")
        .to_string_lossy();
    up.with_file_name(file_name.replace(".up.sql", ".down.sql"))
}

pub async fn apply_sql_file(pool: &PgPool, path: &Path) {
    let sql = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    sqlx::raw_sql(AssertSqlSafe(sql))
        .execute(pool)
        .await
        .unwrap_or_else(|error| panic!("apply {}: {error}", path.display()));
}

pub async fn apply_all_up_migrations(pool: &PgPool) {
    for path in up_migration_files() {
        apply_sql_file(pool, &path).await;
    }
}
