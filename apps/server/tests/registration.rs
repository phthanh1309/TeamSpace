//! Integration tests for `POST /auth/register`.
//!
//! The tests need a running PostgreSQL instance configured through
//! `DATABASE_URL` (loaded from the workspace `.env` when present). Each test
//! works in its own temporary schema and never touches the `public` schema.
//! When `DATABASE_URL` is not set the tests report a skip instead of failing.

mod support;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header::CONTENT_TYPE};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use server::password::verify_password;
use sqlx::PgPool;
use support::{TestSchema, apply_all_up_migrations, database_url};
use tower::ServiceExt;

const PASSWORD: &str = "correct horse battery staple";

struct RegistrationResponse {
    status: StatusCode,
    body: String,
    json: Value,
}

async fn register(app: &Router, body: Value) -> RegistrationResponse {
    register_raw(app, body.to_string()).await
}

async fn register_raw(app: &Router, body: String) -> RegistrationResponse {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .expect("build the registration request"),
        )
        .await
        .expect("send the registration request");

    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read the registration response body");
    let text = String::from_utf8(bytes.to_vec()).expect("response body is UTF-8");
    let json = if text.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&text).expect("registration response is JSON")
    };

    RegistrationResponse {
        status,
        body: text,
        json,
    }
}

async fn user_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(pool)
        .await
        .expect("count users")
}

#[tokio::test]
async fn valid_registration_returns_201_and_persists_a_normalized_user() {
    let Some(url) = database_url() else {
        eprintln!("DATABASE_URL is not set; skipping registration integration test");
        return;
    };

    let schema = TestSchema::new(&url).await;
    apply_all_up_migrations(&schema.pool).await;
    let app = server::app(schema.pool.clone());

    let response = register(
        &app,
        json!({
            "email": "  Alice@Example.COM  ",
            "password": PASSWORD,
            "display_name": "  Alice  ",
        }),
    )
    .await;

    assert_eq!(response.status, StatusCode::CREATED, "{}", response.body);
    assert_eq!(response.json["email"], "alice@example.com");
    assert_eq!(response.json["display_name"], "Alice");
    assert!(
        response.json["id"].as_i64().is_some_and(|id| id > 0),
        "response must contain a positive id: {}",
        response.body
    );

    let created_at = response.json["created_at"]
        .as_str()
        .expect("response must contain created_at");
    let created_at = DateTime::parse_from_rfc3339(created_at)
        .expect("created_at must be RFC 3339")
        .with_timezone(&Utc);

    for secret in ["password", "password_hash", PASSWORD, "$argon2"] {
        assert!(
            !response.body.contains(secret),
            "response must not expose {secret:?}: {}",
            response.body
        );
    }

    let id = response.json["id"].as_i64().expect("numeric id");
    let (stored_email, stored_hash, stored_display_name, stored_created_at) =
        sqlx::query_as::<_, (String, String, String, DateTime<Utc>)>(
            "SELECT email, password_hash, display_name, created_at FROM users WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&schema.pool)
        .await
        .expect("read the stored user");

    assert_eq!(stored_email, "alice@example.com");
    assert_eq!(stored_display_name, "Alice");
    assert_ne!(
        stored_hash, PASSWORD,
        "plaintext passwords must never be stored"
    );
    assert!(
        stored_hash.starts_with("$argon2id$"),
        "stored hash must be Argon2id: {stored_hash}"
    );
    assert!(verify_password(PASSWORD, &stored_hash));
    assert_eq!(stored_created_at, created_at);

    schema.cleanup().await;
}

#[tokio::test]
async fn duplicate_email_with_different_casing_returns_409() {
    let Some(url) = database_url() else {
        eprintln!("DATABASE_URL is not set; skipping duplicate email test");
        return;
    };

    let schema = TestSchema::new(&url).await;
    apply_all_up_migrations(&schema.pool).await;
    let app = server::app(schema.pool.clone());

    let first = register(
        &app,
        json!({
            "email": "alice@example.com",
            "password": PASSWORD,
            "display_name": "Alice",
        }),
    )
    .await;
    assert_eq!(first.status, StatusCode::CREATED, "{}", first.body);

    let duplicate = register(
        &app,
        json!({
            "email": "  ALICE@Example.COM ",
            "password": "another valid password",
            "display_name": "Alice Again",
        }),
    )
    .await;

    assert_eq!(duplicate.status, StatusCode::CONFLICT, "{}", duplicate.body);
    assert_eq!(duplicate.json["error"], "email_taken");
    assert!(
        !duplicate.body.contains("23505") && !duplicate.body.contains("duplicate key"),
        "raw database details must not leak: {}",
        duplicate.body
    );

    assert_eq!(user_count(&schema.pool).await, 1);

    schema.cleanup().await;
}

#[tokio::test]
async fn invalid_requests_return_400_and_create_no_user() {
    let Some(url) = database_url() else {
        eprintln!("DATABASE_URL is not set; skipping invalid request test");
        return;
    };

    let schema = TestSchema::new(&url).await;
    apply_all_up_migrations(&schema.pool).await;
    let app = server::app(schema.pool.clone());

    let cases = [
        json!({
            "email": "   ",
            "password": PASSWORD,
            "display_name": "Alice",
        }),
        json!({
            "email": "alice@example.com",
            "password": "short",
            "display_name": "Alice",
        }),
        json!({
            "email": "alice@example.com",
            "password": PASSWORD,
            "display_name": "   ",
        }),
        json!({
            "email": "alice@example.com",
            "password": PASSWORD,
        }),
    ];

    for case in cases {
        let response = register(&app, case).await;
        assert_eq!(
            response.status,
            StatusCode::BAD_REQUEST,
            "{}",
            response.body
        );
        assert_eq!(response.json["error"], "invalid_request");
    }

    let malformed = register_raw(&app, "{ not json".to_owned()).await;
    assert_eq!(malformed.status, StatusCode::BAD_REQUEST);
    assert_eq!(malformed.json["error"], "invalid_request");

    assert_eq!(user_count(&schema.pool).await, 0);

    schema.cleanup().await;
}
