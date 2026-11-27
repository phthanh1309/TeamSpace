//! Integration tests for project membership.
//!
//! The tests need a running PostgreSQL instance configured through
//! `DATABASE_URL` (loaded from the workspace `.env` when present). Each test
//! works in its own temporary schema and never touches the `public` schema.
//! When `DATABASE_URL` is not set the tests report a skip instead of failing.

mod support;

use axum::Router;
use axum::http::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;
use support::{
    ApiResponse, TestSchema, apply_all_up_migrations, apply_sql_file, create_project, database_url,
    down_migration_file, register_and_login, send, send_raw, setup, up_migration_files,
};

fn assert_no_internal_details(body: &str) {
    for fragment in [
        "SQLSTATE",
        "duplicate key",
        "password",
        "password_hash",
        support::TEST_PASSWORD,
    ] {
        assert!(
            !body.contains(fragment),
            "response must not expose {fragment:?}: {body}"
        );
    }
}

async fn add_member(app: &Router, token: &str, project_id: i64, email: &str) -> ApiResponse {
    send(
        app,
        "POST",
        &format!("/projects/{project_id}/members"),
        Some(json!({ "email": email })),
        Some(token),
    )
    .await
}

async fn list_members(app: &Router, token: &str, project_id: i64) -> ApiResponse {
    send(
        app,
        "GET",
        &format!("/projects/{project_id}/members"),
        None,
        Some(token),
    )
    .await
}

async fn remove_member(app: &Router, token: &str, project_id: i64, user_id: i64) -> ApiResponse {
    send(
        app,
        "DELETE",
        &format!("/projects/{project_id}/members/{user_id}"),
        None,
        Some(token),
    )
    .await
}

fn roles(response: &ApiResponse) -> Vec<String> {
    response.json["members"]
        .as_array()
        .expect("members array")
        .iter()
        .map(|member| member["role"].as_str().expect("string role").to_owned())
        .collect()
}

fn member_for(response: &ApiResponse, user_id: i64) -> Value {
    response.json["members"]
        .as_array()
        .expect("members array")
        .iter()
        .find(|member| member["user_id"] == user_id)
        .unwrap_or_else(|| panic!("user {user_id} must be listed: {}", response.body))
        .clone()
}

async fn membership_count(pool: &PgPool, project_id: i64) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM project_members WHERE project_id = $1")
        .bind(project_id)
        .fetch_one(pool)
        .await
        .expect("count memberships")
}

#[tokio::test]
async fn owner_is_listed_as_the_only_member() {
    let Some((schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping member listing test");
        return;
    };

    let (owner_id, owner_token) = register_and_login(&app, "alice@example.com", "Alice").await;
    let project_id = create_project(&app, &owner_token, "Capstone").await;

    let response = list_members(&app, &owner_token, project_id).await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.body);
    assert_no_internal_details(&response.body);

    assert_eq!(roles(&response), vec!["owner".to_owned()]);
    let owner = member_for(&response, owner_id);
    assert_eq!(owner["email"], "alice@example.com");
    assert_eq!(owner["display_name"], "Alice");
    assert!(
        owner["joined_at"].is_string(),
        "joined_at must be present: {}",
        response.body
    );
    assert_eq!(membership_count(&schema.pool, project_id).await, 0);
}

#[tokio::test]
async fn member_can_read_the_project_and_its_members() {
    let Some((_schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping member read test");
        return;
    };

    let (owner_id, owner_token) = register_and_login(&app, "alice@example.com", "Alice").await;
    let (member_id, member_token) = register_and_login(&app, "bob@example.com", "Bob").await;
    let project_id = create_project(&app, &owner_token, "Capstone").await;

    let added = add_member(&app, &owner_token, project_id, "bob@example.com").await;
    assert_eq!(added.status, StatusCode::CREATED, "{}", added.body);

    let project = send(
        &app,
        "GET",
        &format!("/projects/{project_id}"),
        None,
        Some(&member_token),
    )
    .await;
    assert_eq!(project.status, StatusCode::OK, "{}", project.body);
    assert_eq!(project.json["owner_id"], owner_id);
    assert_eq!(project.json["name"], "Capstone");

    let members = list_members(&app, &member_token, project_id).await;
    assert_eq!(members.status, StatusCode::OK, "{}", members.body);
    assert_eq!(
        roles(&members),
        vec!["owner".to_owned(), "member".to_owned()]
    );
    assert_eq!(member_for(&members, member_id)["email"], "bob@example.com");
}

#[tokio::test]
async fn outsider_cannot_read_the_project_or_its_members() {
    let Some((_schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping outsider access test");
        return;
    };

    let (_owner_id, owner_token) = register_and_login(&app, "alice@example.com", "Alice").await;
    let (_outsider_id, outsider_token) =
        register_and_login(&app, "carol@example.com", "Carol").await;
    let project_id = create_project(&app, &owner_token, "Capstone").await;

    let members = list_members(&app, &outsider_token, project_id).await;
    assert_eq!(members.status, StatusCode::NOT_FOUND, "{}", members.body);
    assert_no_internal_details(&members.body);

    let project = send(
        &app,
        "GET",
        &format!("/projects/{project_id}"),
        None,
        Some(&outsider_token),
    )
    .await;
    assert_eq!(project.status, StatusCode::NOT_FOUND, "{}", project.body);

    let added = add_member(&app, &outsider_token, project_id, "bob@example.com").await;
    assert_eq!(added.status, StatusCode::NOT_FOUND, "{}", added.body);
}

#[tokio::test]
async fn owner_adds_a_registered_user_by_email() {
    let Some((schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping add member test");
        return;
    };

    let (_owner_id, owner_token) = register_and_login(&app, "alice@example.com", "Alice").await;
    let (member_id, _member_token) = register_and_login(&app, "bob@example.com", "Bob").await;
    let project_id = create_project(&app, &owner_token, "Capstone").await;

    // The lookup must be case-insensitive and ignore surrounding whitespace.
    let response = add_member(&app, &owner_token, project_id, "  BOB@Example.com ").await;
    assert_eq!(response.status, StatusCode::CREATED, "{}", response.body);
    assert_no_internal_details(&response.body);

    assert_eq!(response.json["user_id"], member_id);
    assert_eq!(response.json["email"], "bob@example.com");
    assert_eq!(response.json["display_name"], "Bob");
    assert_eq!(response.json["role"], "member");
    assert!(
        response.json["joined_at"].is_string(),
        "joined_at must be present: {}",
        response.body
    );

    assert_eq!(membership_count(&schema.pool, project_id).await, 1);

    let members = list_members(&app, &owner_token, project_id).await;
    assert_eq!(
        roles(&members),
        vec!["owner".to_owned(), "member".to_owned()]
    );
}

#[tokio::test]
async fn adding_an_unknown_email_returns_404() {
    let Some((schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping unknown email test");
        return;
    };

    let (_owner_id, owner_token) = register_and_login(&app, "alice@example.com", "Alice").await;
    let project_id = create_project(&app, &owner_token, "Capstone").await;

    let response = add_member(&app, &owner_token, project_id, "nobody@example.com").await;
    assert_eq!(response.status, StatusCode::NOT_FOUND, "{}", response.body);
    assert_no_internal_details(&response.body);
    assert_eq!(membership_count(&schema.pool, project_id).await, 0);
}

#[tokio::test]
async fn duplicate_membership_is_rejected() {
    let Some((schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping duplicate membership test");
        return;
    };

    let (_owner_id, owner_token) = register_and_login(&app, "alice@example.com", "Alice").await;
    let (_member_id, _member_token) = register_and_login(&app, "bob@example.com", "Bob").await;
    let project_id = create_project(&app, &owner_token, "Capstone").await;

    let first = add_member(&app, &owner_token, project_id, "bob@example.com").await;
    assert_eq!(first.status, StatusCode::CREATED, "{}", first.body);

    let second = add_member(&app, &owner_token, project_id, "bob@example.com").await;
    assert_eq!(second.status, StatusCode::CONFLICT, "{}", second.body);
    assert_no_internal_details(&second.body);
    assert_eq!(membership_count(&schema.pool, project_id).await, 1);
}

#[tokio::test]
async fn the_owner_cannot_be_added_as_a_member() {
    let Some((schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping owner-as-member test");
        return;
    };

    let (_owner_id, owner_token) = register_and_login(&app, "alice@example.com", "Alice").await;
    let project_id = create_project(&app, &owner_token, "Capstone").await;

    let response = add_member(&app, &owner_token, project_id, "alice@example.com").await;
    assert_eq!(response.status, StatusCode::CONFLICT, "{}", response.body);
    assert_no_internal_details(&response.body);
    assert_eq!(membership_count(&schema.pool, project_id).await, 0);
}

#[tokio::test]
async fn members_cannot_manage_members() {
    let Some((_schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping member permissions test");
        return;
    };

    let (owner_id, owner_token) = register_and_login(&app, "alice@example.com", "Alice").await;
    let (member_id, member_token) = register_and_login(&app, "bob@example.com", "Bob").await;
    let (other_id, _other_token) = register_and_login(&app, "carol@example.com", "Carol").await;
    let project_id = create_project(&app, &owner_token, "Capstone").await;

    let added = add_member(&app, &owner_token, project_id, "bob@example.com").await;
    assert_eq!(added.status, StatusCode::CREATED, "{}", added.body);

    let add = add_member(&app, &member_token, project_id, "carol@example.com").await;
    assert_eq!(add.status, StatusCode::FORBIDDEN, "{}", add.body);
    assert_no_internal_details(&add.body);

    let remove = remove_member(&app, &member_token, project_id, other_id).await;
    assert_eq!(remove.status, StatusCode::FORBIDDEN, "{}", remove.body);

    let remove_self = remove_member(&app, &member_token, project_id, member_id).await;
    assert_eq!(
        remove_self.status,
        StatusCode::FORBIDDEN,
        "{}",
        remove_self.body
    );

    // Nothing changed: the owner and Bob are still the only participants.
    let members = list_members(&app, &owner_token, project_id).await;
    assert_eq!(
        roles(&members),
        vec!["owner".to_owned(), "member".to_owned()]
    );
    assert!(member_for(&members, owner_id)["role"] == "owner");
}

#[tokio::test]
async fn members_cannot_update_or_delete_the_project() {
    let Some((_schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping member write permissions test");
        return;
    };

    let (_owner_id, owner_token) = register_and_login(&app, "alice@example.com", "Alice").await;
    let (_member_id, member_token) = register_and_login(&app, "bob@example.com", "Bob").await;
    let project_id = create_project(&app, &owner_token, "Capstone").await;
    let added = add_member(&app, &owner_token, project_id, "bob@example.com").await;
    assert_eq!(added.status, StatusCode::CREATED, "{}", added.body);

    let update = send(
        &app,
        "PATCH",
        &format!("/projects/{project_id}"),
        Some(json!({ "name": "Hijacked" })),
        Some(&member_token),
    )
    .await;
    assert_eq!(update.status, StatusCode::FORBIDDEN, "{}", update.body);
    assert_no_internal_details(&update.body);

    let delete = send(
        &app,
        "DELETE",
        &format!("/projects/{project_id}"),
        None,
        Some(&member_token),
    )
    .await;
    assert_eq!(delete.status, StatusCode::FORBIDDEN, "{}", delete.body);

    // The owner still sees the original project.
    let project = send(
        &app,
        "GET",
        &format!("/projects/{project_id}"),
        None,
        Some(&owner_token),
    )
    .await;
    assert_eq!(project.status, StatusCode::OK, "{}", project.body);
    assert_eq!(project.json["name"], "Capstone");
}

#[tokio::test]
async fn project_list_includes_memberships_once() {
    let Some((_schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping project list test");
        return;
    };

    let (_owner_id, owner_token) = register_and_login(&app, "alice@example.com", "Alice").await;
    let (_member_id, member_token) = register_and_login(&app, "bob@example.com", "Bob").await;
    let (_outsider_id, outsider_token) =
        register_and_login(&app, "carol@example.com", "Carol").await;

    let shared_id = create_project(&app, &owner_token, "Shared").await;
    let owned_id = create_project(&app, &owner_token, "Owner only").await;
    let added = add_member(&app, &owner_token, shared_id, "bob@example.com").await;
    assert_eq!(added.status, StatusCode::CREATED, "{}", added.body);

    let member_list = send(&app, "GET", "/projects", None, Some(&member_token)).await;
    assert_eq!(member_list.status, StatusCode::OK, "{}", member_list.body);
    let projects = member_list.json["projects"]
        .as_array()
        .expect("projects array");
    assert_eq!(
        projects.len(),
        1,
        "a member must see the shared project exactly once: {}",
        member_list.body
    );
    assert_eq!(projects[0]["id"], shared_id);

    let owner_list = send(&app, "GET", "/projects", None, Some(&owner_token)).await;
    let projects = owner_list.json["projects"]
        .as_array()
        .expect("projects array");
    assert_eq!(projects.len(), 2);
    assert_eq!(
        projects
            .iter()
            .filter(|project| project["id"] == shared_id)
            .count(),
        1,
        "the owner must not see the shared project twice: {}",
        owner_list.body
    );
    assert!(
        projects.iter().any(|project| project["id"] == owned_id),
        "the owner still sees their own projects"
    );

    let outsider_list = send(&app, "GET", "/projects", None, Some(&outsider_token)).await;
    assert_eq!(
        outsider_list.status,
        StatusCode::OK,
        "{}",
        outsider_list.body
    );
    assert_eq!(
        outsider_list.json["projects"]
            .as_array()
            .expect("projects array")
            .len(),
        0
    );
}

#[tokio::test]
async fn the_owner_cannot_remove_themselves() {
    let Some((schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping owner removal test");
        return;
    };

    let (owner_id, owner_token) = register_and_login(&app, "alice@example.com", "Alice").await;
    let project_id = create_project(&app, &owner_token, "Capstone").await;

    let response = remove_member(&app, &owner_token, project_id, owner_id).await;
    assert_eq!(response.status, StatusCode::CONFLICT, "{}", response.body);
    assert_no_internal_details(&response.body);
    assert_eq!(membership_count(&schema.pool, project_id).await, 0);

    let members = list_members(&app, &owner_token, project_id).await;
    assert_eq!(roles(&members), vec!["owner".to_owned()]);
}

#[tokio::test]
async fn removing_a_member_revokes_access() {
    let Some((schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping member removal test");
        return;
    };

    let (_owner_id, owner_token) = register_and_login(&app, "alice@example.com", "Alice").await;
    let (member_id, member_token) = register_and_login(&app, "bob@example.com", "Bob").await;
    let project_id = create_project(&app, &owner_token, "Capstone").await;
    let added = add_member(&app, &owner_token, project_id, "bob@example.com").await;
    assert_eq!(added.status, StatusCode::CREATED, "{}", added.body);

    let removed = remove_member(&app, &owner_token, project_id, member_id).await;
    assert_eq!(removed.status, StatusCode::OK, "{}", removed.body);
    assert_eq!(removed.json["status"], "removed");
    assert_no_internal_details(&removed.body);
    assert_eq!(membership_count(&schema.pool, project_id).await, 0);

    let project = send(
        &app,
        "GET",
        &format!("/projects/{project_id}"),
        None,
        Some(&member_token),
    )
    .await;
    assert_eq!(project.status, StatusCode::NOT_FOUND, "{}", project.body);

    let members = list_members(&app, &member_token, project_id).await;
    assert_eq!(members.status, StatusCode::NOT_FOUND, "{}", members.body);

    let list = send(&app, "GET", "/projects", None, Some(&member_token)).await;
    assert_eq!(
        list.json["projects"]
            .as_array()
            .expect("projects array")
            .len(),
        0
    );

    let again = remove_member(&app, &owner_token, project_id, member_id).await;
    assert_eq!(again.status, StatusCode::NOT_FOUND, "{}", again.body);
}

#[tokio::test]
async fn member_endpoints_require_authentication() {
    let Some((_schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping member authentication test");
        return;
    };

    let (_owner_id, owner_token) = register_and_login(&app, "alice@example.com", "Alice").await;
    let project_id = create_project(&app, &owner_token, "Capstone").await;

    let cases = [
        ("GET", format!("/projects/{project_id}/members"), None),
        (
            "POST",
            format!("/projects/{project_id}/members"),
            Some(json!({ "email": "bob@example.com" })),
        ),
        ("DELETE", format!("/projects/{project_id}/members/1"), None),
    ];

    for (method, uri, body) in cases {
        let response = send(&app, method, &uri, body, None).await;
        assert_eq!(
            response.status,
            StatusCode::UNAUTHORIZED,
            "{method} {uri} must require a token: {}",
            response.body
        );
    }
}

#[tokio::test]
async fn invalid_member_requests_are_rejected() {
    let Some((_schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping member validation test");
        return;
    };

    let (_owner_id, owner_token) = register_and_login(&app, "alice@example.com", "Alice").await;
    let project_id = create_project(&app, &owner_token, "Capstone").await;

    for body in [json!({ "email": "   " }), json!({ "email": "" })] {
        let response = send(
            &app,
            "POST",
            &format!("/projects/{project_id}/members"),
            Some(body),
            Some(&owner_token),
        )
        .await;
        assert_eq!(
            response.status,
            StatusCode::BAD_REQUEST,
            "{}",
            response.body
        );
    }

    let malformed = send_raw(
        &app,
        "POST",
        &format!("/projects/{project_id}/members"),
        Some("{\"email\":"),
        Some(&owner_token),
    )
    .await;
    assert_eq!(
        malformed.status,
        StatusCode::BAD_REQUEST,
        "{}",
        malformed.body
    );

    let overlong = "a".repeat(300);
    let response = add_member(&app, &owner_token, project_id, &overlong).await;
    assert_eq!(
        response.status,
        StatusCode::BAD_REQUEST,
        "{}",
        response.body
    );

    for uri in [
        "/projects/0/members".to_owned(),
        "/projects/not-a-number/members".to_owned(),
    ] {
        let response = send(&app, "GET", &uri, None, Some(&owner_token)).await;
        assert_eq!(response.status, StatusCode::NOT_FOUND, "{}", response.body);
    }

    for uri in [
        format!("/projects/{project_id}/members/0"),
        format!("/projects/{project_id}/members/not-a-number"),
    ] {
        let response = send(&app, "DELETE", &uri, None, Some(&owner_token)).await;
        assert_eq!(response.status, StatusCode::NOT_FOUND, "{}", response.body);
    }

    let missing_project = add_member(&app, &owner_token, 999_999, "bob@example.com").await;
    assert_eq!(
        missing_project.status,
        StatusCode::NOT_FOUND,
        "{}",
        missing_project.body
    );
}

#[tokio::test]
async fn membership_schema_constraints_and_migration_are_reversible() {
    let Some(url) = database_url() else {
        eprintln!("DATABASE_URL is not set; skipping membership migration test");
        return;
    };

    let schema = TestSchema::new(&url).await;
    apply_all_up_migrations(&schema.pool).await;

    let members_up = up_migration_files()
        .into_iter()
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains("create_project_members"))
        })
        .expect("the membership migration must exist");

    // Two users, one project owned by the first, and one membership row.
    let owner_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (email, password_hash, display_name)
         VALUES ('owner@example.com', 'hash', 'Owner') RETURNING id",
    )
    .fetch_one(&schema.pool)
    .await
    .expect("insert the owner");

    let member_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (email, password_hash, display_name)
         VALUES ('member@example.com', 'hash', 'Member') RETURNING id",
    )
    .fetch_one(&schema.pool)
    .await
    .expect("insert the member");

    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (name, owner_id) VALUES ('Capstone', $1) RETURNING id",
    )
    .bind(owner_id)
    .fetch_one(&schema.pool)
    .await
    .expect("insert the project");

    sqlx::query("INSERT INTO project_members (project_id, user_id) VALUES ($1, $2)")
        .bind(project_id)
        .bind(member_id)
        .execute(&schema.pool)
        .await
        .expect("insert the first membership");

    let duplicate =
        sqlx::query("INSERT INTO project_members (project_id, user_id) VALUES ($1, $2)")
            .bind(project_id)
            .bind(member_id)
            .execute(&schema.pool)
            .await
            .expect_err("a duplicate membership must be rejected");
    support::assert_sqlstate(duplicate, "23505");

    // A user who owns work cannot be deleted, which is the existing policy.
    let owner_delete = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(owner_id)
        .execute(&schema.pool)
        .await
        .expect_err("an owner with projects must not be deletable");
    support::assert_sqlstate(owner_delete, "23503");

    // Deleting the project cascades to its memberships.
    sqlx::query("DELETE FROM projects WHERE id = $1")
        .bind(project_id)
        .execute(&schema.pool)
        .await
        .expect("delete the project");
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM project_members")
        .fetch_one(&schema.pool)
        .await
        .expect("count memberships");
    assert_eq!(remaining, 0, "project deletion must cascade memberships");

    // Deleting a user cascades to their memberships.
    let second_project: i64 = sqlx::query_scalar(
        "INSERT INTO projects (name, owner_id) VALUES ('Second', $1) RETURNING id",
    )
    .bind(owner_id)
    .fetch_one(&schema.pool)
    .await
    .expect("insert a second project");
    sqlx::query("INSERT INTO project_members (project_id, user_id) VALUES ($1, $2)")
        .bind(second_project)
        .bind(member_id)
        .execute(&schema.pool)
        .await
        .expect("insert a membership");
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(member_id)
        .execute(&schema.pool)
        .await
        .expect("delete a member without owned projects");
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM project_members")
        .fetch_one(&schema.pool)
        .await
        .expect("count memberships");
    assert_eq!(remaining, 0, "user deletion must cascade memberships");

    // The down migration removes the table and the up migration restores it.
    let down = down_migration_file(&members_up);
    apply_sql_file(&schema.pool, &down).await;
    let table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1 FROM information_schema.tables
             WHERE table_schema = current_schema() AND table_name = 'project_members'
         )",
    )
    .fetch_one(&schema.pool)
    .await
    .expect("check for the membership table");
    assert!(!table_exists, "the down migration must drop the table");

    apply_sql_file(&schema.pool, &members_up).await;
    let table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1 FROM information_schema.tables
             WHERE table_schema = current_schema() AND table_name = 'project_members'
         )",
    )
    .fetch_one(&schema.pool)
    .await
    .expect("check for the membership table after re-applying");
    assert!(table_exists, "the up migration must be re-applicable");

    schema.cleanup().await;
}
