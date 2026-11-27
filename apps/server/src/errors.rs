//! Client-visible error responses shared by the API modules.
//!
//! Every failure returns the same JSON shape so clients can handle errors
//! uniformly. Messages are static and deliberately vague: no database error
//! text, SQL, parameters or secrets ever reach a client.

use axum::{
    Json,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;

/// Consistent error body for every client-visible failure.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: &'static str,
    pub message: &'static str,
}

/// Builds a JSON error response.
pub fn error_response(status: StatusCode, error: &'static str, message: &'static str) -> Response {
    (status, Json(ErrorResponse { error, message })).into_response()
}

/// The response for any unexpected server-side failure.
pub fn internal_error() -> Response {
    error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "internal_error",
        "an internal server error occurred",
    )
}

/// Rejection for a missing, malformed, expired or revoked bearer token.
pub fn unauthorized() -> Response {
    let mut response = error_response(
        StatusCode::UNAUTHORIZED,
        "unauthorized",
        "a valid bearer token is required",
    );
    response
        .headers_mut()
        .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    response
}

/// Rejection for a request body that could not be parsed or validated.
pub fn invalid_request(message: &'static str) -> Response {
    error_response(StatusCode::BAD_REQUEST, "invalid_request", message)
}

/// The single response for a resource the caller may not see.
///
/// A resource that exists but belongs to another user is reported exactly like
/// a missing one, so guessing identifiers reveals nothing.
pub fn not_found(message: &'static str) -> Response {
    error_response(StatusCode::NOT_FOUND, "not_found", message)
}

/// The response for an action the caller may not perform.
///
/// Used only for resources the caller can already see, such as a project the
/// user belongs to but does not own, so it reveals nothing new.
pub fn forbidden(message: &'static str) -> Response {
    error_response(StatusCode::FORBIDDEN, "forbidden", message)
}

/// The response for a request that conflicts with the current state.
pub fn conflict(message: &'static str) -> Response {
    error_response(StatusCode::CONFLICT, "conflict", message)
}

/// Returns whether the error is a PostgreSQL unique violation (`23505`).
pub fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(|database_error| database_error.code())
        .is_some_and(|code| code.as_ref() == "23505")
}

/// Logs only the SQLSTATE so no SQL text, parameters or credentials leak.
pub fn log_database_error(endpoint: &str, error: &sqlx::Error) {
    let code = error
        .as_database_error()
        .and_then(|database_error| database_error.code())
        .unwrap_or_else(|| "unknown".into());
    eprintln!("{endpoint} failed: database error (SQLSTATE {code})");
}
