//! Typed HTTP client for the TeamSpace backend.
//!
//! Requests go to the Axum server described in `apps/server`. The bearer token
//! is only ever sent in the `Authorization` header and is never logged.
//!
//! The base URL defaults to the local development server and can be overridden
//! at build time with `TEAMSPACE_API_URL`.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// Backend base URL used when `TEAMSPACE_API_URL` is not set at build time.
const DEFAULT_API_BASE_URL: &str = "http://127.0.0.1:3000";

fn api_base_url() -> &'static str {
    match option_env!("TEAMSPACE_API_URL") {
        Some(url) => url,
        None => DEFAULT_API_BASE_URL,
    }
}

/// One pooled client so connections are reused across calls.
fn http() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

/// The signed-in user, as returned by the auth endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub display_name: String,
    pub created_at: String,
}

/// A successful login.
#[derive(Debug, Clone, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    /// Kept so the UI can warn about expiry in a later milestone.
    #[allow(dead_code)]
    pub expires_at: String,
}

/// A project owned by the current user.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
struct ListProjectsResponse {
    projects: Vec<Project>,
}

/// The error body every failing endpoint returns.
#[derive(Debug, Default, Deserialize)]
struct ErrorBody {
    #[serde(default)]
    message: String,
}

/// A failed API call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    status: Option<u16>,
    message: String,
}

impl ApiError {
    fn network() -> Self {
        Self {
            status: None,
            message: String::new(),
        }
    }

    pub fn is_unauthorized(&self) -> bool {
        self.status == Some(401)
    }

    pub fn is_not_found(&self) -> bool {
        self.status == Some(404)
    }

    /// A message safe and useful to show in the UI.
    ///
    /// Validation messages from the API are shown as-is; server and network
    /// failures are replaced with a generic explanation.
    pub fn user_message(&self) -> String {
        match self.status {
            Some(401) => "Your session has expired. Please sign in again.".to_owned(),
            Some(404) => "That item no longer exists.".to_owned(),
            Some(status) if status >= 500 => {
                "The server had a problem. Please try again.".to_owned()
            }
            _ if !self.message.is_empty() => self.message.clone(),
            _ => "Something went wrong. Please try again.".to_owned(),
        }
    }
}

#[derive(Serialize)]
struct RegisterRequest<'a> {
    email: &'a str,
    password: &'a str,
    display_name: &'a str,
}

#[derive(Serialize)]
struct LoginRequest<'a> {
    email: &'a str,
    password: &'a str,
}

#[derive(Serialize)]
struct ProjectWrite<'a> {
    name: &'a str,
    description: Option<&'a str>,
}

async fn read_json<T: serde::de::DeserializeOwned>(
    request: reqwest::RequestBuilder,
) -> Result<T, ApiError> {
    let response = request.send().await.map_err(|_| ApiError::network())?;
    let status = response.status();

    if status.is_success() {
        return response.json::<T>().await.map_err(|_| ApiError::network());
    }

    let body = response.json::<ErrorBody>().await.unwrap_or_default();
    Err(ApiError {
        status: Some(status.as_u16()),
        message: body.message,
    })
}

async fn read_empty(request: reqwest::RequestBuilder) -> Result<(), ApiError> {
    let response = request.send().await.map_err(|_| ApiError::network())?;
    let status = response.status();

    if status.is_success() {
        return Ok(());
    }

    let body = response.json::<ErrorBody>().await.unwrap_or_default();
    Err(ApiError {
        status: Some(status.as_u16()),
        message: body.message,
    })
}

/// Creates an account and returns the stored user.
pub async fn register(email: &str, password: &str, display_name: &str) -> Result<User, ApiError> {
    read_json(
        http()
            .post(format!("{}/auth/register", api_base_url()))
            .json(&RegisterRequest {
                email,
                password,
                display_name,
            }),
    )
    .await
}

/// Exchanges credentials for a bearer token.
pub async fn login(email: &str, password: &str) -> Result<LoginResponse, ApiError> {
    read_json(
        http()
            .post(format!("{}/auth/login", api_base_url()))
            .json(&LoginRequest { email, password }),
    )
    .await
}

/// Loads the user that owns the current session.
pub async fn me(token: &str) -> Result<User, ApiError> {
    read_json(
        http()
            .get(format!("{}/auth/me", api_base_url()))
            .bearer_auth(token),
    )
    .await
}

/// Revokes the current session server-side.
pub async fn logout(token: &str) -> Result<(), ApiError> {
    read_empty(
        http()
            .post(format!("{}/auth/logout", api_base_url()))
            .bearer_auth(token),
    )
    .await
}

/// Lists the projects the current user owns.
pub async fn list_projects(token: &str) -> Result<Vec<Project>, ApiError> {
    let response: ListProjectsResponse = read_json(
        http()
            .get(format!("{}/projects", api_base_url()))
            .bearer_auth(token),
    )
    .await?;

    Ok(response.projects)
}

/// Creates a project. A blank description is sent as `null`.
pub async fn create_project(
    token: &str,
    name: &str,
    description: Option<&str>,
) -> Result<Project, ApiError> {
    read_json(
        http()
            .post(format!("{}/projects", api_base_url()))
            .bearer_auth(token)
            .json(&ProjectWrite { name, description }),
    )
    .await
}

/// Loads one project.
pub async fn get_project(token: &str, project_id: i64) -> Result<Project, ApiError> {
    read_json(
        http()
            .get(format!("{}/projects/{project_id}", api_base_url()))
            .bearer_auth(token),
    )
    .await
}

/// Replaces the editable fields of a project.
///
/// `description: None` clears the description on the server.
pub async fn update_project(
    token: &str,
    project_id: i64,
    name: &str,
    description: Option<&str>,
) -> Result<Project, ApiError> {
    read_json(
        http()
            .patch(format!("{}/projects/{project_id}", api_base_url()))
            .bearer_auth(token)
            .json(&ProjectWrite { name, description }),
    )
    .await
}

/// Deletes a project.
pub async fn delete_project(token: &str, project_id: i64) -> Result<(), ApiError> {
    read_empty(
        http()
            .delete(format!("{}/projects/{project_id}", api_base_url()))
            .bearer_auth(token),
    )
    .await
}
