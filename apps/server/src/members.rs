//! Project membership endpoints.
//!
//! The owner is never stored in `project_members`; the owner role is derived
//! from `projects.owner_id` and reported alongside the extra `member` rows, so
//! the two can never disagree.
//!
//! Every handler is behind [`AuthUser`]. Reading members requires access to the
//! project (owner or member); changing them requires ownership.

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
use domain::normalize_email;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

use crate::auth::{AuthUser, MAX_EMAIL_LENGTH};
use crate::errors::{
    conflict, forbidden, internal_error, invalid_request, is_unique_violation, log_database_error,
    not_found,
};
use crate::projects::{Project, find_accessible_project, parse_project_id};
use crate::users::find_user_id_by_email;

/// A project participant as returned by the members endpoints.
///
/// `role` is `owner` or `member`. It is computed in SQL so the owner row and
/// member rows share one response shape.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Member {
    pub user_id: i64,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub joined_at: DateTime<Utc>,
}

/// Response of `GET /projects/{project_id}/members`.
#[derive(Debug, Serialize)]
pub struct ListMembersResponse {
    pub members: Vec<Member>,
}

/// Response of `DELETE /projects/{project_id}/members/{user_id}`.
#[derive(Debug, Serialize)]
pub struct RemoveMemberResponse {
    pub status: &'static str,
}

/// Body of `POST /projects/{project_id}/members`.
#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub email: String,
}

fn project_not_found() -> Response {
    not_found("project not found")
}

/// Loads a project the caller may see, or the response to return instead.
async fn require_project_access(
    pool: &PgPool,
    endpoint: &str,
    project_id: i64,
    user_id: i64,
) -> Result<Project, Response> {
    match find_accessible_project(pool, project_id, user_id).await {
        Ok(Some(project)) => Ok(project),
        Ok(None) => Err(project_not_found()),
        Err(error) => {
            log_database_error(endpoint, &error);
            Err(internal_error())
        }
    }
}

/// Members are managed by the owner only.
fn require_owner(project: &Project, user_id: i64) -> Result<(), Response> {
    if project.owner_id == user_id {
        Ok(())
    } else {
        Err(forbidden("only the project owner can manage members"))
    }
}

/// Parses both path ids, treating malformed or non-positive ids as not found.
fn parse_member_path(
    path: Result<Path<(i64, i64)>, PathRejection>,
) -> Result<(i64, i64), Response> {
    match path {
        Ok(Path((project_id, user_id))) if project_id > 0 && user_id > 0 => {
            Ok((project_id, user_id))
        }
        _ => Err(project_not_found()),
    }
}

/// Lists the owner first, then every member in the order they joined.
async fn list_project_members(pool: &PgPool, project_id: i64) -> Result<Vec<Member>, sqlx::Error> {
    let owner = sqlx::query_as::<_, Member>(
        r#"
        SELECT p.owner_id AS user_id,
               u.email,
               u.display_name,
               'owner'::text AS role,
               p.created_at AS joined_at
        FROM projects p
        JOIN users u ON u.id = p.owner_id
        WHERE p.id = $1
        "#,
    )
    .bind(project_id)
    .fetch_optional(pool)
    .await?;

    let members = sqlx::query_as::<_, Member>(
        r#"
        SELECT m.user_id,
               u.email,
               u.display_name,
               'member'::text AS role,
               m.created_at AS joined_at
        FROM project_members m
        JOIN users u ON u.id = m.user_id
        WHERE m.project_id = $1
        ORDER BY m.created_at ASC, m.user_id ASC
        "#,
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;

    let mut all = owner.into_iter().collect::<Vec<_>>();
    all.extend(members);
    Ok(all)
}

/// Inserts a membership and returns it joined with the user's profile.
///
/// A duplicate raises the primary-key unique violation, which the handler turns
/// into a `409` instead of an internal error.
async fn insert_member(
    pool: &PgPool,
    project_id: i64,
    user_id: i64,
) -> Result<Member, sqlx::Error> {
    sqlx::query_as::<_, Member>(
        r#"
        WITH inserted AS (
            INSERT INTO project_members (project_id, user_id)
            VALUES ($1, $2)
            RETURNING user_id, created_at
        )
        SELECT i.user_id,
               u.email,
               u.display_name,
               'member'::text AS role,
               i.created_at AS joined_at
        FROM inserted i
        JOIN users u ON u.id = i.user_id
        "#,
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
}

async fn delete_member(pool: &PgPool, project_id: i64, user_id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM project_members
        WHERE project_id = $1
          AND user_id = $2
        "#,
    )
    .bind(project_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Handles `GET /projects/{project_id}/members`.
pub async fn list_members(
    State(pool): State<PgPool>,
    auth: AuthUser,
    path: Result<Path<i64>, PathRejection>,
) -> Response {
    let project_id = match parse_project_id(path) {
        Ok(project_id) => project_id,
        Err(response) => return response,
    };

    if let Err(response) = require_project_access(
        &pool,
        "GET /projects/{project_id}/members",
        project_id,
        auth.id,
    )
    .await
    {
        return response;
    }

    match list_project_members(&pool, project_id).await {
        Ok(members) => (StatusCode::OK, Json(ListMembersResponse { members })).into_response(),
        Err(error) => {
            log_database_error("GET /projects/{project_id}/members", &error);
            internal_error()
        }
    }
}

/// Handles `POST /projects/{project_id}/members`.
///
/// Adds an already registered user by email. There is no invitation email in
/// this milestone.
pub async fn add_member(
    State(pool): State<PgPool>,
    auth: AuthUser,
    path: Result<Path<i64>, PathRejection>,
    payload: Result<Json<AddMemberRequest>, JsonRejection>,
) -> Response {
    let project_id = match parse_project_id(path) {
        Ok(project_id) => project_id,
        Err(response) => return response,
    };

    let project = match require_project_access(
        &pool,
        "POST /projects/{project_id}/members",
        project_id,
        auth.id,
    )
    .await
    {
        Ok(project) => project,
        Err(response) => return response,
    };
    if let Err(response) = require_owner(&project, auth.id) {
        return response;
    }

    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(_) => return invalid_request("request body must be valid JSON with an email"),
    };

    let email = normalize_email(&request.email);
    if email.is_empty() {
        return invalid_request("email must not be empty");
    }
    if email.chars().count() > MAX_EMAIL_LENGTH {
        return invalid_request("email must be at most 255 characters");
    }

    let user_id = match find_user_id_by_email(&pool, &email).await {
        Ok(Some(user_id)) => user_id,
        Ok(None) => return not_found("no account uses that email"),
        Err(error) => {
            log_database_error("POST /projects/{project_id}/members", &error);
            return internal_error();
        }
    };

    if user_id == project.owner_id {
        return conflict("that user already owns this project");
    }

    match insert_member(&pool, project_id, user_id).await {
        Ok(member) => (StatusCode::CREATED, Json(member)).into_response(),
        Err(error) if is_unique_violation(&error) => {
            conflict("that user is already a member of this project")
        }
        Err(error) => {
            log_database_error("POST /projects/{project_id}/members", &error);
            internal_error()
        }
    }
}

/// Handles `DELETE /projects/{project_id}/members/{user_id}`. Owner only.
pub async fn remove_member(
    State(pool): State<PgPool>,
    auth: AuthUser,
    path: Result<Path<(i64, i64)>, PathRejection>,
) -> Response {
    let (project_id, user_id) = match parse_member_path(path) {
        Ok(ids) => ids,
        Err(response) => return response,
    };

    let project = match require_project_access(
        &pool,
        "DELETE /projects/{project_id}/members/{user_id}",
        project_id,
        auth.id,
    )
    .await
    {
        Ok(project) => project,
        Err(response) => return response,
    };
    if let Err(response) = require_owner(&project, auth.id) {
        return response;
    }

    if user_id == project.owner_id {
        return conflict("the project owner cannot be removed");
    }

    match delete_member(&pool, project_id, user_id).await {
        Ok(true) => (
            StatusCode::OK,
            Json(RemoveMemberResponse { status: "removed" }),
        )
            .into_response(),
        Ok(false) => not_found("member not found"),
        Err(error) => {
            log_database_error("DELETE /projects/{project_id}/members/{user_id}", &error);
            internal_error()
        }
    }
}
