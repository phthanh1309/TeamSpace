//! Integration tests for `POST /auth/login`, `POST /auth/logout` and
//! `GET /auth/me`.
//!
//! The tests need a running PostgreSQL instance configured through
//! `DATABASE_URL` (loaded from the workspace `.env` when present). Each test
//! works in its own temporary schema and never touches the `public` schema.
//! When `DATABASE_URL` is not set the tests report a skip instead of failing.

mod support;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{
    HeaderMap, Request, StatusCode,
    header::{AUTHORIZATION, CONTENT_TYPE, WWW_AUTHENTICATE},
};
use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use server::sessions::hash_token;
use sqlx::PgPool;
use support::{TestSchema, apply_all_up_migrations, database_url};
use tower::ServiceExt;

const PASSWORD: &str = "correct horse battery staple";

struct ApiResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
    json: Value,
}

async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
    bearer: Option<&str>,
) -> ApiResponse {
    let mut builder = Request::builder().method(method).uri(uri);

    if body.is_some() {
        builder = builder.header(CONTENT_TYPE, "application/json");
    }
    if let Some(token) = bearer {
        builder = builder.header(AUTHORIZATION, format!("Bearer {token}"));
    }

    let body = body.map(|value| value.to_string()).unwrap_or_default();
    let response = app
        .clone()
        .oneshot(
            builder
                .body(Body::from(body))
                .expect("build the authentication request"),
        )
        .await
        .expect("send the authentication request");

    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read the authentication response body");
    let text = String::from_utf8(bytes.to_vec()).expect("response body is UTF-8");
    let json = if text.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&text).expect("authentication response is JSON")
    };

    ApiResponse {
        status,
        headers,
        body: text,
        json,
    }
}

async fn setup() -> Option<(TestSchema, Router)> {
    let url = database_url()?;
    let schema = TestSchema::new(&url).await;
    apply_all_up_migrations(&schema.pool).await;
    let app = server::app(schema.pool.clone());
    Some((schema, app))
}

async fn register(app: &Router, email: &str, display_name: &str) -> ApiResponse {
    let response = send(
        app,
        "POST",
        "/auth/register",
        Some(json!({
            "email": email,
            "password": PASSWORD,
            "display_name": display_name,
        })),
        None,
    )
    .await;

    assert_eq!(response.status, StatusCode::CREATED, "{}", response.body);
    response
}

async fn login(app: &Router, email: &str, password: &str) -> ApiResponse {
    send(
        app,
        "POST",
        "/auth/login",
        Some(json!({ "email": email, "password": password })),
        None,
    )
    .await
}

async fn session_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM sessions")
        .fetch_one(pool)
        .await
        .expect("count sessions")
}

fn assert_no_secrets(body: &str) {
    for secret in ["password", "password_hash", "$argon2", PASSWORD] {
        assert!(
            !body.contains(secret),
            "response must not expose {secret:?}: {body}"
        );
    }
}

#[tokio::test]
async fn successful_login_returns_a_token_and_stores_only_its_hash() {
    let Some((schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping login integration test");
        return;
    };

    let registered = register(&app, "  Alice@Example.COM  ", "Alice").await;
    let user_id = registered.json["id"].as_i64().expect("numeric user id");

    let response = login(&app, "  ALICE@example.COM ", PASSWORD).await;

    assert_eq!(response.status, StatusCode::OK, "{}", response.body);
    assert_no_secrets(&response.body);

    let token = response.json["token"]
        .as_str()
        .expect("login response must contain a token")
        .to_owned();
    assert_eq!(token.len(), 64, "32 random bytes must be hex encoded");
    assert!(
        token.chars().all(|character| character.is_ascii_hexdigit()),
        "token must be hexadecimal: {token}"
    );

    let expires_at = response.json["expires_at"]
        .as_str()
        .expect("login response must contain expires_at");
    let expires_at = DateTime::parse_from_rfc3339(expires_at)
        .expect("expires_at must be RFC 3339")
        .with_timezone(&Utc);
    assert!(expires_at > Utc::now());
    assert!(expires_at <= Utc::now() + Duration::days(8));

    let (stored_user_id, stored_hash, stored_created_at, stored_expires_at) =
        sqlx::query_as::<_, (i64, String, DateTime<Utc>, DateTime<Utc>)>(
            "SELECT user_id, token_hash, created_at, expires_at FROM sessions",
        )
        .fetch_one(&schema.pool)
        .await
        .expect("read the stored session");

    assert_eq!(stored_user_id, user_id);
    assert_eq!(stored_hash, hash_token(&token));
    assert_ne!(stored_hash, token, "the raw token must never be stored");
    assert_eq!(
        stored_expires_at.timestamp_micros(),
        expires_at.timestamp_micros(),
        "the returned expiry must match the persisted expiry"
    );
    assert_eq!(
        stored_expires_at - stored_created_at,
        Duration::days(7),
        "sessions must live for seven days"
    );

    let rows_with_raw_token: i64 =
        sqlx::query_scalar("SELECT count(*) FROM sessions WHERE token_hash = $1")
            .bind(&token)
            .fetch_one(&schema.pool)
            .await
            .expect("look up the raw token");
    assert_eq!(rows_with_raw_token, 0);

    schema.cleanup().await;
}

#[tokio::test]
async fn wrong_password_and_unknown_email_return_the_same_generic_error() {
    let Some((schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping generic login error test");
        return;
    };

    register(&app, "alice@example.com", "Alice").await;

    let wrong_password = login(&app, "alice@example.com", "definitely wrong").await;
    let unknown_email = login(&app, "nobody@example.com", PASSWORD).await;

    assert_eq!(
        wrong_password.status,
        StatusCode::UNAUTHORIZED,
        "{}",
        wrong_password.body
    );
    assert_eq!(
        unknown_email.status,
        StatusCode::UNAUTHORIZED,
        "{}",
        unknown_email.body
    );
    assert_eq!(wrong_password.json, unknown_email.json);
    assert_eq!(wrong_password.json["error"], "invalid_credentials");
    assert!(
        !wrong_password.body.contains("alice")
            && !unknown_email.body.contains("nobody")
            && !wrong_password.body.contains("not found"),
        "the generic error must not reveal account state"
    );

    assert_eq!(session_count(&schema.pool).await, 0);

    schema.cleanup().await;
}

#[tokio::test]
async fn me_requires_a_valid_token_and_returns_safe_user_fields() {
    let Some((schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping /auth/me test");
        return;
    };

    let registered = register(&app, "alice@example.com", "Alice").await;
    let user_id = registered.json["id"].as_i64().expect("numeric user id");
    let created_at = registered.json["created_at"].clone();

    let missing = send(&app, "GET", "/auth/me", None, None).await;
    assert_eq!(missing.status, StatusCode::UNAUTHORIZED, "{}", missing.body);
    assert_eq!(missing.json["error"], "unauthorized");
    assert_eq!(
        missing.headers.get(WWW_AUTHENTICATE),
        Some(&"Bearer".parse().expect("static header value"))
    );

    let invalid = send(&app, "GET", "/auth/me", None, Some("not-a-real-token")).await;
    assert_eq!(invalid.status, StatusCode::UNAUTHORIZED, "{}", invalid.body);

    let logged_in = login(&app, "alice@example.com", PASSWORD).await;
    assert_eq!(logged_in.status, StatusCode::OK, "{}", logged_in.body);
    let token = logged_in.json["token"]
        .as_str()
        .expect("login token")
        .to_owned();

    let me = send(&app, "GET", "/auth/me", None, Some(&token)).await;
    assert_eq!(me.status, StatusCode::OK, "{}", me.body);
    assert_eq!(me.json["id"], user_id);
    assert_eq!(me.json["email"], "alice@example.com");
    assert_eq!(me.json["display_name"], "Alice");
    assert_eq!(me.json["created_at"], created_at);
    assert_no_secrets(&me.body);
    assert!(
        !me.body.contains(&token) && !me.body.contains("token"),
        "the session token must not be echoed: {}",
        me.body
    );

    schema.cleanup().await;
}

#[tokio::test]
async fn expired_session_is_rejected() {
    let Some((schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping expired session test");
        return;
    };

    register(&app, "alice@example.com", "Alice").await;

    let logged_in = login(&app, "alice@example.com", PASSWORD).await;
    assert_eq!(logged_in.status, StatusCode::OK, "{}", logged_in.body);
    let token = logged_in.json["token"]
        .as_str()
        .expect("login token")
        .to_owned();

    sqlx::query("UPDATE sessions SET expires_at = $1 WHERE token_hash = $2")
        .bind(Utc::now() - Duration::seconds(1))
        .bind(hash_token(&token))
        .execute(&schema.pool)
        .await
        .expect("expire the session");

    let me = send(&app, "GET", "/auth/me", None, Some(&token)).await;
    assert_eq!(me.status, StatusCode::UNAUTHORIZED, "{}", me.body);
    assert_eq!(me.json["error"], "unauthorized");

    schema.cleanup().await;
}

#[tokio::test]
async fn logout_revokes_the_current_session() {
    let Some((schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping logout test");
        return;
    };

    register(&app, "alice@example.com", "Alice").await;

    let logged_in = login(&app, "alice@example.com", PASSWORD).await;
    assert_eq!(logged_in.status, StatusCode::OK, "{}", logged_in.body);
    let token = logged_in.json["token"]
        .as_str()
        .expect("login token")
        .to_owned();

    let before = send(&app, "GET", "/auth/me", None, Some(&token)).await;
    assert_eq!(before.status, StatusCode::OK, "{}", before.body);

    let logout = send(&app, "POST", "/auth/logout", None, Some(&token)).await;
    assert_eq!(logout.status, StatusCode::OK, "{}", logout.body);
    assert_eq!(logout.json["status"], "logged_out");
    assert_eq!(session_count(&schema.pool).await, 0);

    let after = send(&app, "GET", "/auth/me", None, Some(&token)).await;
    assert_eq!(after.status, StatusCode::UNAUTHORIZED, "{}", after.body);

    let second_logout = send(&app, "POST", "/auth/logout", None, Some(&token)).await;
    assert_eq!(
        second_logout.status,
        StatusCode::UNAUTHORIZED,
        "{}",
        second_logout.body
    );

    schema.cleanup().await;
}

#[tokio::test]
async fn logout_requires_authentication() {
    let Some((schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping logout authentication test");
        return;
    };

    let response = send(&app, "POST", "/auth/logout", None, None).await;

    assert_eq!(
        response.status,
        StatusCode::UNAUTHORIZED,
        "{}",
        response.body
    );
    assert_eq!(response.json["error"], "unauthorized");
    assert_eq!(session_count(&schema.pool).await, 0);

    schema.cleanup().await;
}
