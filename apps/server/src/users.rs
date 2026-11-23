//! User persistence helpers.
//!
//! Email addresses are always normalized before they reach the database so that
//! stored rows and lookups agree with the case-insensitive unique index on
//! `lower(email)`.

use domain::normalize_email;
use sqlx::PgPool;

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
    let email = normalize_email(email);

    sqlx::query_scalar::<_, i64>(
        r#"
        INSERT INTO users (email, password_hash, display_name)
        VALUES ($1, $2, $3)
        RETURNING id
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
