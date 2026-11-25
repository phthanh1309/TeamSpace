//! User persistence helpers.
//!
//! Email addresses are always normalized before they reach the database so that
//! stored rows and lookups agree with the case-insensitive unique index on
//! `lower(email)`.

use chrono::{DateTime, Utc};
use domain::normalize_email;
use sqlx::{FromRow, PgPool};

/// A user row as stored in PostgreSQL, without the password hash.
#[derive(Debug, Clone, FromRow)]
pub struct CreatedUser {
    pub id: i64,
    pub email: String,
    pub display_name: String,
    pub created_at: DateTime<Utc>,
}

/// Inserts a user and returns its id.
///
/// The email is trimmed and lowercased before it is persisted. A duplicate
/// address that differs only by casing is rejected by the database.
pub async fn create_user(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
    display_name: &str,
) -> Result<i64, sqlx::Error> {
    create_user_returning(pool, email, password_hash, display_name)
        .await
        .map(|user| user.id)
}

/// Inserts a user and returns the stored row.
///
/// The email is normalized exactly like [`create_user`]. `created_at` is
/// assigned by the database so callers can return the persisted timestamp.
pub async fn create_user_returning(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
    display_name: &str,
) -> Result<CreatedUser, sqlx::Error> {
    let email = normalize_email(email);

    sqlx::query_as::<_, CreatedUser>(
        r#"
        INSERT INTO users (email, password_hash, display_name)
        VALUES ($1, $2, $3)
        RETURNING id, email, display_name, created_at
        "#,
    )
    .bind(email)
    .bind(password_hash)
    .bind(display_name)
    .fetch_one(pool)
    .await
}

/// Looks up a user id by email, normalizing the lookup value first.
///
/// The query filters on `lower(email)` so it can use the case-insensitive
/// unique index.
pub async fn find_user_id_by_email(pool: &PgPool, email: &str) -> Result<Option<i64>, sqlx::Error> {
    let email = normalize_email(email);

    sqlx::query_scalar::<_, i64>("SELECT id FROM users WHERE lower(email) = $1")
        .bind(email)
        .fetch_optional(pool)
        .await
}

/// A user row including the password hash, for the login flow only.
///
/// The hash must stay inside the authentication code: it is never serialized
/// or returned to a client.
#[derive(Debug, Clone, FromRow)]
pub struct UserCredentials {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
    pub display_name: String,
    pub created_at: DateTime<Utc>,
}

/// Looks up the credentials for a normalized email address.
///
/// Like [`find_user_id_by_email`], the lookup is case-insensitive.
pub async fn find_user_credentials_by_email(
    pool: &PgPool,
    email: &str,
) -> Result<Option<UserCredentials>, sqlx::Error> {
    let email = normalize_email(email);

    sqlx::query_as::<_, UserCredentials>(
        r#"
        SELECT id, email, password_hash, display_name, created_at
        FROM users
        WHERE lower(email) = $1
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await
}
