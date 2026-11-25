//! Registration, login, logout and current-user endpoints.
//!
//! `POST /auth/register` validates the request, hashes the password and
//! persists the user through [`crate::users`]. `POST /auth/login` verifies the
//! credentials and issues an opaque server-side session through
//! [`crate::sessions`]. [`AuthUser`] extracts and validates the bearer token
//! for protected endpoints such as `GET /auth/me` and `POST /auth/logout`.
//!
//! No handler or error path logs passwords, session tokens or the
//! `Authorization` header.

use std::sync::OnceLock;

use axum::{
    Json,
    extract::{FromRequestParts, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use domain::normalize_email;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::errors::{
    error_response, internal_error, is_unique_violation, log_database_error, unauthorized,
};
use crate::password::{MAX_PASSWORD_LENGTH, MIN_PASSWORD_LENGTH};
use crate::{password, sessions, users};

/// Largest email accepted, matching `users.email VARCHAR(255)`.
pub const MAX_EMAIL_LENGTH: usize = 255;

/// Largest display name accepted, matching `users.display_name VARCHAR(100)`.
pub const MAX_DISPLAY_NAME_LENGTH: usize = 100;

/// Body of `POST /auth/register`.
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub display_name: String,
}

/// Successful registration response.
///
/// Deliberately contains no password or password hash.
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub id: i64,
    pub email: String,
    pub display_name: String,
    pub created_at: DateTime<Utc>,
}

/// Body of `POST /auth/login`.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Successful login response.
///
/// The raw token is returned exactly once. Only its hash is persisted, so this
/// response cannot be reconstructed from the database later.
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

/// Safe user fields returned by `GET /auth/me`.
///
/// Never includes `password_hash` or the session token.
#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub id: i64,
    pub email: String,
    pub display_name: String,
    pub created_at: DateTime<Utc>,
}

/// Successful logout response.
#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub status: &'static str,
}

/// The authenticated user identity and the session that proved it.
///
/// Extracted from `Authorization: Bearer <token>`. The raw token is discarded
/// immediately after hashing; only the session id is retained.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: i64,
    pub email: String,
    pub display_name: String,
    pub created_at: DateTime<Utc>,
    pub session_id: i64,
}

/// A request that passed validation.
#[derive(Debug)]
struct ValidRegistration {
    email: String,
    password: String,
    display_name: String,
}

/// Reason a registration request is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValidationError {
    EmailEmpty,
    EmailTooLong,
    DisplayNameEmpty,
    DisplayNameTooLong,
    PasswordTooShort,
    PasswordTooLong,
}

impl ValidationError {
    fn message(self) -> &'static str {
        match self {
            Self::EmailEmpty => "email must not be empty",
            Self::EmailTooLong => "email must be at most 255 characters",
            Self::DisplayNameEmpty => "display_name must not be empty",
            Self::DisplayNameTooLong => "display_name must be at most 100 characters",
            Self::PasswordTooShort => "password must be at least 8 characters",
            Self::PasswordTooLong => "password must be at most 1024 characters",
        }
    }
}

impl RegisterRequest {
    fn validate(self) -> Result<ValidRegistration, ValidationError> {
        let email = normalize_email(&self.email);
        if email.is_empty() {
            return Err(ValidationError::EmailEmpty);
        }
        if email.chars().count() > MAX_EMAIL_LENGTH {
            return Err(ValidationError::EmailTooLong);
        }

        let display_name = self.display_name.trim().to_owned();
        if display_name.is_empty() {
            return Err(ValidationError::DisplayNameEmpty);
        }
        if display_name.chars().count() > MAX_DISPLAY_NAME_LENGTH {
            return Err(ValidationError::DisplayNameTooLong);
        }

        // Passwords are validated as given: trimming would silently change a
        // credential the user chose.
        let password_length = self.password.chars().count();
        if password_length < MIN_PASSWORD_LENGTH {
            return Err(ValidationError::PasswordTooShort);
        }
        if password_length > MAX_PASSWORD_LENGTH {
            return Err(ValidationError::PasswordTooLong);
        }

        Ok(ValidRegistration {
            email,
            password: self.password,
            display_name,
        })
    }
}

/// The single error returned for both an unknown email and a wrong password.
///
/// Keeping one message and status avoids disclosing whether an account exists.
fn invalid_credentials() -> Response {
    error_response(
        StatusCode::UNAUTHORIZED,
        "invalid_credentials",
        "invalid email or password",
    )
}

/// Handles `POST /auth/register`.
pub async fn register(
    State(pool): State<PgPool>,
    payload: Result<Json<RegisterRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "request body must be valid JSON with email, password and display_name",
            );
        }
    };

    let registration = match request.validate() {
        Ok(registration) => registration,
        Err(error) => {
            return error_response(StatusCode::BAD_REQUEST, "invalid_request", error.message());
        }
    };

    let ValidRegistration {
        email,
        password,
        display_name,
    } = registration;

    let password_hash =
        match tokio::task::spawn_blocking(move || password::hash_password(&password)).await {
            Ok(Ok(hash)) => hash,
            Ok(Err(_)) => {
                eprintln!("POST /auth/register failed: password hashing error");
                return internal_error();
            }
            Err(_) => {
                eprintln!("POST /auth/register failed: password hashing task failed");
                return internal_error();
            }
        };

    match users::create_user_returning(&pool, &email, &password_hash, &display_name).await {
        Ok(user) => (
            StatusCode::CREATED,
            Json(RegisterResponse {
                id: user.id,
                email: user.email,
                display_name: user.display_name,
                created_at: user.created_at,
            }),
        )
            .into_response(),
        Err(error) if is_unique_violation(&error) => error_response(
            StatusCode::CONFLICT,
            "email_taken",
            "an account with this email already exists",
        ),
        Err(error) => {
            log_database_error("POST /auth/register", &error);
            internal_error()
        }
    }
}

/// Returns a valid Argon2id hash used only to spend comparable CPU when the
/// supplied email does not exist.
///
/// Without this, an unknown email would return much faster than a wrong
/// password, leaking account existence through timing.
fn dummy_password_hash() -> &'static str {
    static DUMMY_HASH: OnceLock<String> = OnceLock::new();
    DUMMY_HASH.get_or_init(|| {
        password::hash_password("teamspace-dummy-login-password").unwrap_or_default()
    })
}

/// Extracts the token from an `Authorization: Bearer <token>` header.
///
/// Returns `None` for a missing header, a non-Bearer scheme, an empty token or
/// a token longer than [`sessions::MAX_TOKEN_LENGTH`]. The header value is
/// never logged.
fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;

    if !scheme.eq_ignore_ascii_case("Bearer") {
        return None;
    }

    let token = token.trim();
    if token.is_empty()
        || token.len() > sessions::MAX_TOKEN_LENGTH
        || token.chars().any(char::is_whitespace)
    {
        return None;
    }

    Some(token)
}

impl FromRequestParts<PgPool> for AuthUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &PgPool,
    ) -> Result<Self, Self::Rejection> {
        let token = bearer_token(&parts.headers).ok_or_else(unauthorized)?;
        let token_hash = sessions::hash_token(token);

        match sessions::find_live_session(state, &token_hash, Utc::now()).await {
            Ok(Some(session)) => Ok(Self {
                id: session.id,
                email: session.email,
                display_name: session.display_name,
                created_at: session.created_at,
                session_id: session.session_id,
            }),
            Ok(None) => Err(unauthorized()),
            Err(error) => {
                log_database_error("authentication", &error);
                Err(internal_error())
            }
        }
    }
}

/// Handles `POST /auth/login`.
///
/// Unknown email and wrong password both return [`invalid_credentials`]. The
/// password check runs on a blocking thread so Argon2 never stalls the Tokio
/// runtime.
pub async fn login(
    State(pool): State<PgPool>,
    payload: Result<Json<LoginRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "request body must be valid JSON with email and password",
            );
        }
    };

    let email = normalize_email(&request.email);
    if email.is_empty() || request.password.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "email and password must not be empty",
        );
    }
    if email.chars().count() > MAX_EMAIL_LENGTH {
        return error_response(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "email must be at most 255 characters",
        );
    }
    if request.password.chars().count() > MAX_PASSWORD_LENGTH {
        return error_response(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "password must be at most 1024 characters",
        );
    }

    let credentials = match users::find_user_credentials_by_email(&pool, &email).await {
        Ok(credentials) => credentials,
        Err(error) => {
            log_database_error("POST /auth/login", &error);
            return internal_error();
        }
    };

    // Verify against the stored hash when the account exists, otherwise against
    // a dummy hash so the work (and response time) is comparable.
    let (user, hash_to_verify) = match credentials {
        Some(credentials) => {
            let hash = credentials.password_hash.clone();
            (Some(credentials), hash)
        }
        None => (None, dummy_password_hash().to_owned()),
    };

    let password = request.password;
    let verified = match tokio::task::spawn_blocking(move || {
        password::verify_password(&password, &hash_to_verify)
    })
    .await
    {
        Ok(verified) => verified,
        Err(_) => {
            eprintln!("POST /auth/login failed: password verification task failed");
            return internal_error();
        }
    };

    let Some(user) = user.filter(|_| verified) else {
        return invalid_credentials();
    };

    let now = Utc::now();
    let expires_at = sessions::expires_at(now);
    let token = match sessions::generate_token() {
        Ok(token) => token,
        Err(_) => {
            eprintln!("POST /auth/login failed: session token generation error");
            return internal_error();
        }
    };
    let token_hash = sessions::hash_token(&token);

    match sessions::create_session(&pool, user.id, &token_hash, now, expires_at).await {
        Ok(_) => (StatusCode::OK, Json(LoginResponse { token, expires_at })).into_response(),
        Err(error) => {
            log_database_error("POST /auth/login", &error);
            internal_error()
        }
    }
}

/// Handles `POST /auth/logout`.
///
/// The extractor guarantees a live session; the row is deleted so the same
/// token is rejected afterwards.
pub async fn logout(State(pool): State<PgPool>, auth: AuthUser) -> Response {
    match sessions::delete_session(&pool, auth.session_id).await {
        Ok(_) => (
            StatusCode::OK,
            Json(LogoutResponse {
                status: "logged_out",
            }),
        )
            .into_response(),
        Err(error) => {
            log_database_error("POST /auth/logout", &error);
            internal_error()
        }
    }
}

/// Handles `GET /auth/me`.
pub async fn me(auth: AuthUser) -> Json<MeResponse> {
    Json(MeResponse {
        id: auth.id,
        email: auth.email,
        display_name: auth.display_name,
        created_at: auth.created_at,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_DISPLAY_NAME_LENGTH, MAX_EMAIL_LENGTH, MIN_PASSWORD_LENGTH, RegisterRequest,
        ValidationError, bearer_token,
    };
    use crate::sessions::MAX_TOKEN_LENGTH;
    use axum::http::{HeaderMap, HeaderValue, header};

    fn authorization_header(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::try_from(value).expect("a valid header value"),
        );
        headers
    }

    #[test]
    fn bearer_token_is_extracted() {
        let headers = authorization_header("Bearer abc123");

        assert_eq!(bearer_token(&headers), Some("abc123"));
    }

    #[test]
    fn bearer_scheme_is_case_insensitive() {
        let headers = authorization_header("bEaReR abc123");

        assert_eq!(bearer_token(&headers), Some("abc123"));
    }

    #[test]
    fn missing_authorization_header_is_rejected() {
        assert_eq!(bearer_token(&HeaderMap::new()), None);
    }

    #[test]
    fn non_bearer_scheme_is_rejected() {
        let headers = authorization_header("Basic abc123");

        assert_eq!(bearer_token(&headers), None);
    }

    #[test]
    fn empty_token_is_rejected() {
        assert_eq!(bearer_token(&authorization_header("Bearer   ")), None);
    }

    #[test]
    fn overlong_token_is_rejected() {
        let value = format!("Bearer {}", "a".repeat(MAX_TOKEN_LENGTH + 1));

        assert_eq!(bearer_token(&authorization_header(&value)), None);
    }

    #[test]
    fn token_with_internal_whitespace_is_rejected() {
        assert_eq!(bearer_token(&authorization_header("Bearer abc def")), None);
    }

    fn request(email: &str, password: &str, display_name: &str) -> RegisterRequest {
        RegisterRequest {
            email: email.to_owned(),
            password: password.to_owned(),
            display_name: display_name.to_owned(),
        }
    }

    fn validate(
        email: &str,
        password: &str,
        display_name: &str,
    ) -> Result<super::ValidRegistration, ValidationError> {
        request(email, password, display_name).validate()
    }

    #[test]
    fn valid_request_is_normalized_and_trimmed() {
        let registration =
            validate("  Alice@Example.COM ", "password123", "  Alice  ").expect("a valid request");

        assert_eq!(registration.email, "alice@example.com");
        assert_eq!(registration.display_name, "Alice");
        assert_eq!(registration.password, "password123");
    }

    #[test]
    fn empty_email_is_rejected() {
        assert_eq!(
            validate("", "password123", "Alice").unwrap_err(),
            ValidationError::EmailEmpty
        );
    }

    #[test]
    fn whitespace_only_email_is_rejected() {
        assert_eq!(
            validate("   \t\n", "password123", "Alice").unwrap_err(),
            ValidationError::EmailEmpty
        );
    }

    #[test]
    fn overlong_email_is_rejected() {
        let email = format!("{}@example.com", "a".repeat(MAX_EMAIL_LENGTH));

        assert_eq!(
            validate(&email, "password123", "Alice").unwrap_err(),
            ValidationError::EmailTooLong
        );
    }

    #[test]
    fn email_at_the_maximum_length_is_accepted() {
        let local = "a".repeat(MAX_EMAIL_LENGTH - "@example.com".len());
        let email = format!("{local}@example.com");
        assert_eq!(email.chars().count(), MAX_EMAIL_LENGTH);

        assert!(validate(&email, "password123", "Alice").is_ok());
    }

    #[test]
    fn empty_display_name_is_rejected_after_trimming() {
        assert_eq!(
            validate("alice@example.com", "password123", "   ").unwrap_err(),
            ValidationError::DisplayNameEmpty
        );
    }

    #[test]
    fn overlong_display_name_is_rejected() {
        let display_name = "a".repeat(MAX_DISPLAY_NAME_LENGTH + 1);

        assert_eq!(
            validate("alice@example.com", "password123", &display_name).unwrap_err(),
            ValidationError::DisplayNameTooLong
        );
    }

    #[test]
    fn display_name_at_the_maximum_length_is_accepted() {
        let display_name = "a".repeat(MAX_DISPLAY_NAME_LENGTH);

        let registration = validate("alice@example.com", "password123", &display_name)
            .expect("100 characters must fit the database column");
        assert_eq!(
            registration.display_name.chars().count(),
            MAX_DISPLAY_NAME_LENGTH
        );
    }

    #[test]
    fn short_password_is_rejected() {
        let password = "a".repeat(MIN_PASSWORD_LENGTH - 1);

        assert_eq!(
            validate("alice@example.com", &password, "Alice").unwrap_err(),
            ValidationError::PasswordTooShort
        );
    }

    #[test]
    fn minimum_length_password_is_accepted() {
        let password = "a".repeat(MIN_PASSWORD_LENGTH);

        assert!(validate("alice@example.com", &password, "Alice").is_ok());
    }

    #[test]
    fn empty_password_is_rejected() {
        assert_eq!(
            validate("alice@example.com", "", "Alice").unwrap_err(),
            ValidationError::PasswordTooShort
        );
    }

    #[test]
    fn overlong_password_is_rejected_instead_of_truncated() {
        let password = "a".repeat(super::MAX_PASSWORD_LENGTH + 1);

        assert_eq!(
            validate("alice@example.com", &password, "Alice").unwrap_err(),
            ValidationError::PasswordTooLong
        );
    }
}
