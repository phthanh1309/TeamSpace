//! Project CRUD endpoints.
//!
//! Every handler is behind [`AuthUser`]. Ownership is stored only in
//! `projects.owner_id`; additional collaborators live in `project_members` with
//! the `member` role, so the owner role is always derived from the column and
//! can never disagree with a duplicate row.
//!
//! Access rules:
//!
//! - the owner and any member may read a project and its members;
//! - only the owner may update or delete it;
//! - a user with no access gets `404`, exactly like a missing project, so
//!   identifiers do not leak existence. A member attempting an owner-only
//!   action gets `403`, because they already know the project exists.

use axum::{
    Json,
    extract::{
        Path, State,
        rejection::{JsonRejection, PathRejection},
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::{FromRow, PgPool};

use crate::auth::AuthUser;
use crate::errors::{forbidden, internal_error, invalid_request, log_database_error, not_found};

/// Largest project name accepted, matching `projects.name VARCHAR(120)`.
pub const MAX_PROJECT_NAME_LENGTH: usize = 120;

/// Largest description accepted.
///
/// `projects.description` is `TEXT`; the cap keeps request bodies bounded
/// instead of letting a client store an arbitrarily large payload.
pub const MAX_PROJECT_DESCRIPTION_LENGTH: usize = 2000;

/// A project row as stored in PostgreSQL.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Response of `GET /projects`.
#[derive(Debug, Serialize)]
pub struct ListProjectsResponse {
    pub projects: Vec<Project>,
}

/// Response of `DELETE /projects/{project_id}`.
#[derive(Debug, Serialize)]
pub struct DeleteProjectResponse {
    pub status: &'static str,
}

/// Body of `POST /projects`.
#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Body of `PATCH /projects/{project_id}`.
///
/// Both fields are optional. `description: null` clears the description while
/// omitting it leaves the stored value unchanged; [`deserialize_optional_field`]
/// keeps those two cases distinct. `name: null` is treated as omitted because a
/// project must always have a name.
#[derive(Debug, Deserialize)]
pub struct UpdateProjectRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_field")]
    pub description: Option<Option<String>>,
}

/// Distinguishes a missing field from an explicit `null` for `Option<Option<T>>`.
///
/// Plain serde maps both to `None`, which would make clearing the description
/// impossible. Wrapping the deserialized value in `Some` keeps the distinction.
fn deserialize_optional_field<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// A create request that passed validation.
#[derive(Debug)]
struct ValidProject {
    name: String,
    description: Option<String>,
}

/// An update request that passed validation.
#[derive(Debug)]
struct ValidProjectUpdate {
    name: Option<String>,
    /// `Some(None)` clears the description, `None` leaves it unchanged.
    description: Option<Option<String>>,
}

/// Reason a project request is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValidationError {
    NameEmpty,
    NameTooLong,
    DescriptionTooLong,
    EmptyUpdate,
}

impl ValidationError {
    fn message(self) -> &'static str {
        match self {
            Self::NameEmpty => "name must not be empty",
            Self::NameTooLong => "name must be at most 120 characters",
            Self::DescriptionTooLong => "description must be at most 2000 characters",
            Self::EmptyUpdate => "provide at least one of name or description",
        }
    }
}

/// Trims and validates a project name.
fn validate_name(name: &str) -> Result<String, ValidationError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ValidationError::NameEmpty);
    }
    if name.chars().count() > MAX_PROJECT_NAME_LENGTH {
        return Err(ValidationError::NameTooLong);
    }

    Ok(name.to_owned())
}

/// Trims a description, mapping blank input to "no description".
fn normalize_description(description: Option<String>) -> Result<Option<String>, ValidationError> {
    let Some(description) = description else {
        return Ok(None);
    };

    let description = description.trim();
    if description.chars().count() > MAX_PROJECT_DESCRIPTION_LENGTH {
        return Err(ValidationError::DescriptionTooLong);
    }
    if description.is_empty() {
        return Ok(None);
    }

    Ok(Some(description.to_owned()))
}

impl CreateProjectRequest {
    fn validate(self) -> Result<ValidProject, ValidationError> {
        Ok(ValidProject {
            name: validate_name(&self.name)?,
            description: normalize_description(self.description)?,
        })
    }
}

impl UpdateProjectRequest {
    fn validate(self) -> Result<ValidProjectUpdate, ValidationError> {
        let name = self.name.as_deref().map(validate_name).transpose()?;
        let description = match self.description {
            Some(description) => Some(normalize_description(description)?),
            None => None,
        };

        if name.is_none() && description.is_none() {
            return Err(ValidationError::EmptyUpdate);
        }

        Ok(ValidProjectUpdate { name, description })
    }
}

/// The error returned for any project the caller does not own.
fn project_not_found() -> Response {
    not_found("project not found")
}

/// Parses the path id, treating a malformed or non-positive id as not found.
///
/// No project can have such an id, so this matches a missing row and keeps the
/// response shape consistent instead of exposing a path-parsing error.
pub(crate) fn parse_project_id(path: Result<Path<i64>, PathRejection>) -> Result<i64, Response> {
    match path {
        Ok(Path(project_id)) if project_id > 0 => Ok(project_id),
        _ => Err(project_not_found()),
    }
}

async fn insert_project(
    pool: &PgPool,
    owner_id: i64,
    project: &ValidProject,
) -> Result<Project, sqlx::Error> {
    sqlx::query_as::<_, Project>(
        r#"
        INSERT INTO projects (name, description, owner_id)
        VALUES ($1, $2, $3)
        RETURNING id, name, description, owner_id, created_at, updated_at
        "#,
    )
    .bind(&project.name)
    .bind(project.description.as_deref())
    .bind(owner_id)
    .fetch_one(pool)
    .await
}

async fn list_projects_for_user(pool: &PgPool, user_id: i64) -> Result<Vec<Project>, sqlx::Error> {
    // `EXISTS` keeps one row per project even though two paths can grant
    // access, so a project can never appear twice in the list.
    sqlx::query_as::<_, Project>(
        r#"
        SELECT p.id, p.name, p.description, p.owner_id, p.created_at, p.updated_at
        FROM projects p
        WHERE p.owner_id = $1
           OR EXISTS (
                SELECT 1
                FROM project_members m
                WHERE m.project_id = p.id
                  AND m.user_id = $1
           )
        ORDER BY p.created_at DESC, p.id DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

/// Loads a project the caller may read: as its owner or as a member.
pub(crate) async fn find_accessible_project(
    pool: &PgPool,
    project_id: i64,
    user_id: i64,
) -> Result<Option<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>(
        r#"
        SELECT p.id, p.name, p.description, p.owner_id, p.created_at, p.updated_at
        FROM projects p
        WHERE p.id = $1
          AND (
                p.owner_id = $2
                OR EXISTS (
                    SELECT 1
                    FROM project_members m
                    WHERE m.project_id = p.id
                      AND m.user_id = $2
                )
          )
        "#,
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

async fn update_owned_project(
    pool: &PgPool,
    project_id: i64,
    owner_id: i64,
    update: &ValidProjectUpdate,
) -> Result<Option<Project>, sqlx::Error> {
    let (set_description, description) = match &update.description {
        Some(description) => (true, description.as_deref()),
        None => (false, None),
    };

    sqlx::query_as::<_, Project>(
        r#"
        UPDATE projects
        SET name = COALESCE($3, name),
            description = CASE WHEN $4 THEN $5 ELSE description END,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = $1
          AND owner_id = $2
        RETURNING id, name, description, owner_id, created_at, updated_at
        "#,
    )
    .bind(project_id)
    .bind(owner_id)
    .bind(update.name.as_deref())
    .bind(set_description)
    .bind(description)
    .fetch_optional(pool)
    .await
}

async fn delete_owned_project(
    pool: &PgPool,
    project_id: i64,
    owner_id: i64,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM projects
        WHERE id = $1
          AND owner_id = $2
        "#,
    )
    .bind(project_id)
    .bind(owner_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Handles `POST /projects`.
pub async fn create_project(
    State(pool): State<PgPool>,
    auth: AuthUser,
    payload: Result<Json<CreateProjectRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(_) => {
            return invalid_request(
                "request body must be valid JSON with name and an optional description",
            );
        }
    };

    let project = match request.validate() {
        Ok(project) => project,
        Err(error) => return invalid_request(error.message()),
    };

    match insert_project(&pool, auth.id, &project).await {
        Ok(project) => (StatusCode::CREATED, Json(project)).into_response(),
        Err(error) => {
            log_database_error("POST /projects", &error);
            internal_error()
        }
    }
}

/// Handles `GET /projects`.
///
/// Lists every project the user owns or belongs to, once per project.
pub async fn list_projects(State(pool): State<PgPool>, auth: AuthUser) -> Response {
    match list_projects_for_user(&pool, auth.id).await {
        Ok(projects) => (StatusCode::OK, Json(ListProjectsResponse { projects })).into_response(),
        Err(error) => {
            log_database_error("GET /projects", &error);
            internal_error()
        }
    }
}

/// Handles `GET /projects/{project_id}` for the owner or any member.
pub async fn get_project(
    State(pool): State<PgPool>,
    auth: AuthUser,
    path: Result<Path<i64>, PathRejection>,
) -> Response {
    let project_id = match parse_project_id(path) {
        Ok(project_id) => project_id,
        Err(response) => return response,
    };

    match find_accessible_project(&pool, project_id, auth.id).await {
        Ok(Some(project)) => (StatusCode::OK, Json(project)).into_response(),
        Ok(None) => project_not_found(),
        Err(error) => {
            log_database_error("GET /projects/{project_id}", &error);
            internal_error()
        }
    }
}

/// Loads a project for an owner-only action.
///
/// A caller with no access gets the usual `404`; a member, who already knows
/// the project exists, gets `403` instead of a misleading "not found".
async fn require_owned_project(
    pool: &PgPool,
    endpoint: &str,
    project_id: i64,
    user_id: i64,
) -> Result<Project, Response> {
    match find_accessible_project(pool, project_id, user_id).await {
        Ok(Some(project)) if project.owner_id == user_id => Ok(project),
        Ok(Some(_)) => Err(forbidden("only the project owner can change this project")),
        Ok(None) => Err(project_not_found()),
        Err(error) => {
            log_database_error(endpoint, &error);
            Err(internal_error())
        }
    }
}

/// Handles `PATCH /projects/{project_id}`. Owner only.
pub async fn update_project(
    State(pool): State<PgPool>,
    auth: AuthUser,
    path: Result<Path<i64>, PathRejection>,
    payload: Result<Json<UpdateProjectRequest>, JsonRejection>,
) -> Response {
    let project_id = match parse_project_id(path) {
        Ok(project_id) => project_id,
        Err(response) => return response,
    };

    if let Err(response) =
        require_owned_project(&pool, "PATCH /projects/{project_id}", project_id, auth.id).await
    {
        return response;
    }

    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(_) => {
            return invalid_request(
                "request body must be valid JSON with an optional name and description",
            );
        }
    };

    let update = match request.validate() {
        Ok(update) => update,
        Err(error) => return invalid_request(error.message()),
    };

    match update_owned_project(&pool, project_id, auth.id, &update).await {
        Ok(Some(project)) => (StatusCode::OK, Json(project)).into_response(),
        Ok(None) => project_not_found(),
        Err(error) => {
            log_database_error("PATCH /projects/{project_id}", &error);
            internal_error()
        }
    }
}

/// Handles `DELETE /projects/{project_id}`. Owner only.
pub async fn delete_project(
    State(pool): State<PgPool>,
    auth: AuthUser,
    path: Result<Path<i64>, PathRejection>,
) -> Response {
    let project_id = match parse_project_id(path) {
        Ok(project_id) => project_id,
        Err(response) => return response,
    };

    if let Err(response) =
        require_owned_project(&pool, "DELETE /projects/{project_id}", project_id, auth.id).await
    {
        return response;
    }

    match delete_owned_project(&pool, project_id, auth.id).await {
        Ok(true) => (
            StatusCode::OK,
            Json(DeleteProjectResponse { status: "deleted" }),
        )
            .into_response(),
        Ok(false) => project_not_found(),
        Err(error) => {
            log_database_error("DELETE /projects/{project_id}", &error);
            internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CreateProjectRequest, MAX_PROJECT_NAME_LENGTH, UpdateProjectRequest, ValidProjectUpdate,
        ValidationError,
    };

    fn create(name: &str, description: Option<&str>) -> CreateProjectRequest {
        CreateProjectRequest {
            name: name.to_owned(),
            description: description.map(str::to_owned),
        }
    }

    fn update(name: Option<&str>, description: Option<Option<&str>>) -> UpdateProjectRequest {
        UpdateProjectRequest {
            name: name.map(str::to_owned),
            description: description.map(|value| value.map(str::to_owned)),
        }
    }

    #[test]
    fn create_trims_name_and_description() {
        let project = create("  Capstone  ", Some("  Build the thing  "))
            .validate()
            .expect("a valid project");

        assert_eq!(project.name, "Capstone");
        assert_eq!(project.description.as_deref(), Some("Build the thing"));
    }

    #[test]
    fn create_maps_blank_description_to_null() {
        let project = create("Capstone", Some("   \t\n "))
            .validate()
            .expect("a valid project");

        assert_eq!(project.description, None);
    }

    #[test]
    fn create_rejects_a_blank_name() {
        assert_eq!(
            create("   ", None).validate().unwrap_err(),
            ValidationError::NameEmpty
        );
    }

    #[test]
    fn create_rejects_an_overlong_name() {
        let name = "a".repeat(MAX_PROJECT_NAME_LENGTH + 1);

        assert_eq!(
            create(&name, None).validate().unwrap_err(),
            ValidationError::NameTooLong
        );
    }

    #[test]
    fn create_accepts_a_name_at_the_maximum_length() {
        let name = "a".repeat(MAX_PROJECT_NAME_LENGTH);

        assert_eq!(
            create(&name, None).validate().expect("name must fit").name,
            name
        );
    }

    #[test]
    fn create_rejects_an_overlong_description() {
        let description = "a".repeat(super::MAX_PROJECT_DESCRIPTION_LENGTH + 1);

        assert_eq!(
            create("Capstone", Some(&description))
                .validate()
                .unwrap_err(),
            ValidationError::DescriptionTooLong
        );
    }

    #[test]
    fn update_distinguishes_missing_and_null_description() {
        let unchanged = update(Some("Renamed"), None)
            .validate()
            .expect("a name update");
        assert_eq!(unchanged.description, None);

        let cleared = update(None, Some(None))
            .validate()
            .expect("a description clear");
        assert_eq!(cleared.description, Some(None));

        let replaced = update(None, Some(Some("  New  ")))
            .validate()
            .expect("a description update");
        assert_eq!(replaced.description, Some(Some("New".to_owned())));
    }

    #[test]
    fn update_requires_at_least_one_field() {
        assert_eq!(
            update(None, None).validate().unwrap_err(),
            ValidationError::EmptyUpdate
        );
    }

    #[test]
    fn update_rejects_a_blank_name() {
        assert_eq!(
            update(Some("   "), None).validate().unwrap_err(),
            ValidationError::NameEmpty
        );
    }

    #[test]
    fn update_keeps_name_and_description_independent() {
        let ValidProjectUpdate { name, description } = update(Some(" Renamed "), None)
            .validate()
            .expect("a name update");

        assert_eq!(name.as_deref(), Some("Renamed"));
        assert_eq!(description, None);
    }
}
