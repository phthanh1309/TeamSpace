//! Opaque server-side sessions.
//!
//! A login generates a random token, returns it to the client once and stores
//! only its SHA-256 hash. Authenticated requests present the raw token in the
//! `Authorization: Bearer <token>` header; the server hashes it and performs a
//! point lookup on `sessions.token_hash`. Raw tokens are never logged or
//! persisted.

use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};

/// Lifetime of a session issued by `POST /auth/login`.
pub const SESSION_LIFETIME_DAYS: i64 = 7;

/// Number of random bytes in a session token (256 bits of entropy).
pub const TOKEN_BYTES: usize = 32;

/// Longest token accepted from a client, in bytes.
///
/// Tokens issued here are 64 hexadecimal characters. Capping the input keeps
/// hashing work bounded for malformed or hostile requests.
pub const MAX_TOKEN_LENGTH: usize = 128;

/// Generates a new opaque session token as lowercase hexadecimal.
///
/// The token is derived from the operating system's cryptographically secure
/// random number generator.
pub fn generate_token() -> Result<String, getrandom::Error> {
    let mut bytes = [0u8; TOKEN_BYTES];
    getrandom::fill(&mut bytes)?;
    Ok(hex::encode(bytes))
}

/// Hashes a session token for storage and lookup.
///
/// SHA-256 is appropriate because the input is a high-entropy random token,
/// not a low-entropy password.
#[must_use]
pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

/// Returns the expiry timestamp for a session created at `now`.
#[must_use]
pub fn expires_at(now: DateTime<Utc>) -> DateTime<Utc> {
    now + Duration::days(SESSION_LIFETIME_DAYS)
}

/// A live session joined with the owning user's safe fields.
#[derive(Debug, Clone, FromRow)]
pub struct SessionUser {
    pub session_id: i64,
    pub id: i64,
    pub email: String,
    pub display_name: String,
    pub created_at: DateTime<Utc>,
}

/// Inserts a session row and returns its id.
pub async fn create_session(
    pool: &PgPool,
    user_id: i64,
    token_hash: &str,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        r#"
        INSERT INTO sessions (user_id, token_hash, created_at, expires_at)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(created_at)
    .bind(expires_at)
    .fetch_one(pool)
    .await
}

/// Looks up a session by token hash, returning it only while it is unexpired.
///
/// The comparison uses the caller-provided UTC timestamp so tests can
/// deterministically exercise expiry without waiting.
pub async fn find_live_session(
    pool: &PgPool,
    token_hash: &str,
    now: DateTime<Utc>,
) -> Result<Option<SessionUser>, sqlx::Error> {
    sqlx::query_as::<_, SessionUser>(
        r#"
        SELECT s.id AS session_id,
               u.id,
               u.email,
               u.display_name,
               u.created_at
        FROM sessions AS s
        JOIN users AS u ON u.id = s.user_id
        WHERE s.token_hash = $1
          AND s.expires_at > $2
        "#,
    )
    .bind(token_hash)
    .bind(now)
    .fetch_optional(pool)
    .await
}

/// Deletes one session by id and reports whether a row was removed.
pub async fn delete_session(pool: &PgPool, session_id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM sessions WHERE id = $1")
        .bind(session_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_TOKEN_LENGTH, SESSION_LIFETIME_DAYS, TOKEN_BYTES, expires_at, generate_token,
        hash_token,
    };
    use chrono::{Duration, TimeZone, Utc};

    #[test]
    fn generated_tokens_carry_256_bits_and_are_hex_encoded() {
        let token = generate_token().expect("the OS random source must work");

        assert_eq!(token.len(), TOKEN_BYTES * 2);
        assert!(
            token.chars().all(|character| character.is_ascii_hexdigit()),
            "token must be hexadecimal: {token}"
        );
        assert_eq!(hex::decode(&token).expect("valid hex").len(), TOKEN_BYTES);
    }

    #[test]
    fn generated_tokens_are_unique() {
        let first = generate_token().expect("the OS random source must work");
        let second = generate_token().expect("the OS random source must work");

        assert_ne!(first, second);
    }

    #[test]
    fn hash_is_sha256_hex_and_never_the_raw_token() {
        let token = generate_token().expect("the OS random source must work");
        let hash = hash_token(&token);

        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|character| character.is_ascii_hexdigit()));
        assert_ne!(hash, token);
        assert!(!hash.contains(&token));
    }

    #[test]
    fn hash_matches_the_known_sha256_vector() {
        assert_eq!(
            hash_token("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hashing_is_deterministic_and_token_specific() {
        let first = hash_token("first-token");
        let second = hash_token("second-token");

        assert_eq!(first, hash_token("first-token"));
        assert_ne!(first, second);
    }

    #[test]
    fn session_lifetime_is_seven_days() {
        let now = Utc
            .with_ymd_and_hms(2026, 1, 1, 12, 0, 0)
            .single()
            .expect("a valid timestamp");

        assert_eq!(expires_at(now), now + Duration::days(SESSION_LIFETIME_DAYS));
        assert_eq!(expires_at(now), now + Duration::days(7));
    }

    #[test]
    fn token_length_cap_is_larger_than_issued_tokens() {
        assert!(MAX_TOKEN_LENGTH > TOKEN_BYTES * 2);
    }
}
