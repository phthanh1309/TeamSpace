//! Integration tests for the project CRUD endpoints.
//!
//! The tests need a running PostgreSQL instance configured through
//! `DATABASE_URL` (loaded from the workspace `.env` when present). Each test
//! works in its own temporary schema and never touches the `public` schema.
//! When `DATABASE_URL` is not set the tests report a skip instead of failing.

mod support;

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::PgPool;
use support::{
    apply_sql_file, assert_sqlstate, create_project, down_migration_file, register_and_login, send,
    send_raw, setup, up_migration_files,
};

async fn project_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM projects")
        .fetch_one(pool)
        .await
        .expect("count projects")
}

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

async fn insert_project_raw(
    pool: &PgPool,
    name: &str,
    description: Option<&str>,
    owner_id: i64,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        INSERT INTO projects (name, description, owner_id)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
    )
    .bind(name)
    .bind(description)
    .bind(owner_id)
    .fetch_one(pool)
    .await
}

#[tokio::test]
async fn create_project_persists_an_owned_project() {
    let Some((schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping create project test");
        return;
    };

    let (user_id, token) = register_and_login(&app, "alice@example.com", "Alice").await;

    let response = send(
        &app,
        "POST",
        "/projects",
        Some(json!({ "name": "  Capstone  ", "description": "  Build it  " })),
        Some(&token),
    )
    .await;

    assert_eq!(response.status, StatusCode::CREATED, "{}", response.body);
    assert_no_internal_details(&response.body);

    let project_id = response.json["id"].as_i64().expect("numeric project id");
    assert!(project_id > 0);
    assert_eq!(response.json["name"], "Capstone");
    assert_eq!(response.json["description"], "Build it");
    assert_eq!(response.json["owner_id"], user_id);

    let created_at = response.json["created_at"]
        .as_str()
        .expect("created_at must be present");
    DateTime::parse_from_rfc3339(created_at).expect("created_at must be RFC 3339");
    let updated_at = response.json["updated_at"]
        .as_str()
        .expect("updated_at must be present");
    DateTime::parse_from_rfc3339(updated_at).expect("updated_at must be RFC 3339");

    let (stored_name, stored_description, stored_owner): (String, Option<String>, i64) =
        sqlx::query_as("SELECT name, description, owner_id FROM projects WHERE id = $1")
            .bind(project_id)
            .fetch_one(&schema.pool)
            .await
            .expect("read the stored project");
    assert_eq!(stored_name, "Capstone");
    assert_eq!(stored_description.as_deref(), Some("Build it"));
    assert_eq!(stored_owner, user_id);

    // A missing or blank description is stored as NULL, never as an empty string.
    for body in [
        json!({ "name": "Without description" }),
        json!({ "name": "Blank description", "description": "   " }),
    ] {
        let response = send(&app, "POST", "/projects", Some(body), Some(&token)).await;
        assert_eq!(response.status, StatusCode::CREATED, "{}", response.body);
        assert!(
            response.json["description"].is_null(),
            "blank or missing descriptions must serialize as null: {}",
            response.body
        );
    }

    assert_eq!(project_count(&schema.pool).await, 3);

    schema.cleanup().await;
}

#[tokio::test]
async fn invalid_project_requests_are_rejected() {
    let Some((schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping invalid project test");
        return;
    };

    let (_, token) = register_and_login(&app, "alice@example.com", "Alice").await;

    let overlong_name = "a".repeat(server::projects::MAX_PROJECT_NAME_LENGTH + 1);
    let overlong_description = "a".repeat(server::projects::MAX_PROJECT_DESCRIPTION_LENGTH + 1);

    let cases = [
        json!({ "name": "   " }),
        json!({ "name": "\t\n" }),
        json!({ "name": overlong_name }),
        json!({ "name": "Capstone", "description": overlong_description }),
        json!({ "description": "no name" }),
    ];

    for case in cases {
        let response = send(&app, "POST", "/projects", Some(case), Some(&token)).await;
        assert_eq!(
            response.status,
            StatusCode::BAD_REQUEST,
            "{}",
            response.body
        );
        assert_eq!(response.json["error"], "invalid_request");
        assert_no_internal_details(&response.body);
    }

    let malformed = send_raw(&app, "POST", "/projects", Some("{ not json"), Some(&token)).await;
    assert_eq!(
        malformed.status,
        StatusCode::BAD_REQUEST,
        "{}",
        malformed.body
    );

    assert_eq!(project_count(&schema.pool).await, 0);

    schema.cleanup().await;
}

#[tokio::test]
async fn list_returns_only_projects_owned_by_the_caller() {
    let Some((schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping list projects test");
        return;
    };

    let (_, alice_token) = register_and_login(&app, "alice@example.com", "Alice").await;
    let (_, bob_token) = register_and_login(&app, "bob@example.com", "Bob").await;
    let (_, carol_token) = register_and_login(&app, "carol@example.com", "Carol").await;

    for name in ["Alpha", "Beta"] {
        create_project(&app, &alice_token, name).await;
    }
    let bob_project_id = create_project(&app, &bob_token, "Bob's project").await;

    let alice_list = send(&app, "GET", "/projects", None, Some(&alice_token)).await;
    assert_eq!(alice_list.status, StatusCode::OK, "{}", alice_list.body);
    let alice_projects = alice_list.json["projects"]
        .as_array()
        .expect("projects array");
    assert_eq!(alice_projects.len(), 2);
    let names: Vec<&str> = alice_projects
        .iter()
        .map(|project| project["name"].as_str().expect("project name"))
        .collect();
    assert!(
        names.contains(&"Alpha") && names.contains(&"Beta"),
        "unexpected projects: {names:?}"
    );
    assert!(
        !names.contains(&"Bob's project"),
        "a user must never see another user's project"
    );
    assert!(
        alice_projects
            .iter()
            .all(|project| project["id"].as_i64() != Some(bob_project_id)),
        "another user's project id must not leak into the list"
    );

    let bob_list = send(&app, "GET", "/projects", None, Some(&bob_token)).await;
    assert_eq!(bob_list.status, StatusCode::OK, "{}", bob_list.body);
    assert_eq!(
        bob_list.json["projects"]
            .as_array()
            .expect("projects array")
            .len(),
        1
    );

    let carol_list = send(&app, "GET", "/projects", None, Some(&carol_token)).await;
    assert_eq!(carol_list.status, StatusCode::OK, "{}", carol_list.body);
    assert!(
        carol_list.json["projects"]
            .as_array()
            .expect("projects array")
            .is_empty(),
        "a user without projects must get an empty list"
    );

    schema.cleanup().await;
}

#[tokio::test]
async fn owner_can_get_their_project_and_others_get_404() {
    let Some((schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping get project test");
        return;
    };

    let (_, alice_token) = register_and_login(&app, "alice@example.com", "Alice").await;
    let (_, bob_token) = register_and_login(&app, "bob@example.com", "Bob").await;

    let created = send(
        &app,
        "POST",
        "/projects",
        Some(json!({ "name": "Alpha", "description": "Owned by Alice" })),
        Some(&alice_token),
    )
    .await;
    let project_id = created.json["id"].as_i64().expect("numeric project id");
    let uri = format!("/projects/{project_id}");

    let owner_view = send(&app, "GET", &uri, None, Some(&alice_token)).await;
    assert_eq!(owner_view.status, StatusCode::OK, "{}", owner_view.body);
    assert_eq!(owner_view.json["name"], "Alpha");
    assert_eq!(owner_view.json["description"], "Owned by Alice");

    let stranger_view = send(&app, "GET", &uri, None, Some(&bob_token)).await;
    assert_eq!(
        stranger_view.status,
        StatusCode::NOT_FOUND,
        "{}",
        stranger_view.body
    );
    assert_eq!(stranger_view.json["error"], "not_found");
    assert!(
        !stranger_view.body.contains("Alpha"),
        "a forbidden response must not reveal the project: {}",
        stranger_view.body
    );

    let missing = send(&app, "GET", "/projects/999999", None, Some(&alice_token)).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND, "{}", missing.body);

    let malformed = send(
        &app,
        "GET",
        "/projects/not-a-number",
        None,
        Some(&alice_token),
    )
    .await;
    assert_eq!(
        malformed.status,
        StatusCode::NOT_FOUND,
        "{}",
        malformed.body
    );

    schema.cleanup().await;
}

#[tokio::test]
async fn owner_can_update_a_project() {
    let Some((schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping update project test");
        return;
    };

    let (_, token) = register_and_login(&app, "alice@example.com", "Alice").await;

    let created = send(
        &app,
        "POST",
        "/projects",
        Some(json!({ "name": "Alpha", "description": "First draft" })),
        Some(&token),
    )
    .await;
    let project_id = created.json["id"].as_i64().expect("numeric project id");
    let created_at = created.json["created_at"]
        .as_str()
        .expect("created_at")
        .to_owned();
    let uri = format!("/projects/{project_id}");

    let renamed = send(
        &app,
        "PATCH",
        &uri,
        Some(json!({ "name": "  Renamed  " })),
        Some(&token),
    )
    .await;
    assert_eq!(renamed.status, StatusCode::OK, "{}", renamed.body);
    assert_eq!(renamed.json["name"], "Renamed");
    assert_eq!(
        renamed.json["description"], "First draft",
        "omitting description must keep the stored value"
    );
    assert_eq!(
        renamed.json["created_at"], created_at,
        "created_at must not change"
    );

    let redescribed = send(
        &app,
        "PATCH",
        &uri,
        Some(json!({ "description": "  Second draft  " })),
        Some(&token),
    )
    .await;
    assert_eq!(redescribed.status, StatusCode::OK, "{}", redescribed.body);
    assert_eq!(redescribed.json["name"], "Renamed");
    assert_eq!(redescribed.json["description"], "Second draft");

    let cleared = send(
        &app,
        "PATCH",
        &uri,
        Some(json!({ "description": null })),
        Some(&token),
    )
    .await;
    assert_eq!(cleared.status, StatusCode::OK, "{}", cleared.body);
    assert!(
        cleared.json["description"].is_null(),
        "an explicit null must clear the description: {}",
        cleared.body
    );

    let stored: Option<String> =
        sqlx::query_scalar("SELECT description FROM projects WHERE id = $1")
            .bind(project_id)
            .fetch_one(&schema.pool)
            .await
            .expect("read the stored description");
    assert_eq!(stored, None);

    let updated_at = DateTime::parse_from_rfc3339(
        cleared.json["updated_at"]
            .as_str()
            .expect("updated_at must be present"),
    )
    .expect("updated_at must be RFC 3339")
    .with_timezone(&Utc);
    let created_at = DateTime::parse_from_rfc3339(&created_at)
        .expect("created_at must be RFC 3339")
        .with_timezone(&Utc);
    assert!(
        updated_at >= created_at,
        "updated_at must not move before created_at"
    );

    schema.cleanup().await;
}

#[tokio::test]
async fn invalid_update_requests_are_rejected() {
    let Some((schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping invalid update test");
        return;
    };

    let (_, token) = register_and_login(&app, "alice@example.com", "Alice").await;
    let project_id = create_project(&app, &token, "Alpha").await;
    let uri = format!("/projects/{project_id}");

    let cases = [
        json!({}),
        json!({ "name": null }),
        json!({ "name": "   " }),
        json!({ "name": "a".repeat(server::projects::MAX_PROJECT_NAME_LENGTH + 1) }),
        json!({ "description": "a".repeat(server::projects::MAX_PROJECT_DESCRIPTION_LENGTH + 1) }),
    ];

    for case in cases {
        let response = send(&app, "PATCH", &uri, Some(case), Some(&token)).await;
        assert_eq!(
            response.status,
            StatusCode::BAD_REQUEST,
            "{}",
            response.body
        );
        assert_eq!(response.json["error"], "invalid_request");
    }

    let stored_name: String = sqlx::query_scalar("SELECT name FROM projects WHERE id = $1")
        .bind(project_id)
        .fetch_one(&schema.pool)
        .await
        .expect("read the stored name");
    assert_eq!(
        stored_name, "Alpha",
        "rejected updates must not modify the row"
    );

    schema.cleanup().await;
}

#[tokio::test]
async fn non_owner_cannot_update_a_project() {
    let Some((schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping non-owner update test");
        return;
    };

    let (_, alice_token) = register_and_login(&app, "alice@example.com", "Alice").await;
    let (_, bob_token) = register_and_login(&app, "bob@example.com", "Bob").await;
    let project_id = create_project(&app, &alice_token, "Alpha").await;
    let uri = format!("/projects/{project_id}");

    let response = send(
        &app,
        "PATCH",
        &uri,
        Some(json!({ "name": "Hijacked" })),
        Some(&bob_token),
    )
    .await;
    assert_eq!(response.status, StatusCode::NOT_FOUND, "{}", response.body);
    assert_eq!(response.json["error"], "not_found");

    let stored_name: String = sqlx::query_scalar("SELECT name FROM projects WHERE id = $1")
        .bind(project_id)
        .fetch_one(&schema.pool)
        .await
        .expect("read the stored name");
    assert_eq!(stored_name, "Alpha", "a non-owner must not change the row");

    schema.cleanup().await;
}

#[tokio::test]
async fn owner_can_delete_a_project() {
    let Some((schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping delete project test");
        return;
    };

    let (_, token) = register_and_login(&app, "alice@example.com", "Alice").await;
    let project_id = create_project(&app, &token, "Alpha").await;
    let uri = format!("/projects/{project_id}");

    let deleted = send(&app, "DELETE", &uri, None, Some(&token)).await;
    assert_eq!(deleted.status, StatusCode::OK, "{}", deleted.body);
    assert_eq!(deleted.json["status"], "deleted");
    assert_eq!(project_count(&schema.pool).await, 0);

    let again = send(&app, "DELETE", &uri, None, Some(&token)).await;
    assert_eq!(again.status, StatusCode::NOT_FOUND, "{}", again.body);

    let view = send(&app, "GET", &uri, None, Some(&token)).await;
    assert_eq!(view.status, StatusCode::NOT_FOUND, "{}", view.body);

    schema.cleanup().await;
}

#[tokio::test]
async fn non_owner_cannot_delete_a_project() {
    let Some((schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping non-owner delete test");
        return;
    };

    let (_, alice_token) = register_and_login(&app, "alice@example.com", "Alice").await;
    let (_, bob_token) = register_and_login(&app, "bob@example.com", "Bob").await;
    let project_id = create_project(&app, &alice_token, "Alpha").await;
    let uri = format!("/projects/{project_id}");

    let response = send(&app, "DELETE", &uri, None, Some(&bob_token)).await;
    assert_eq!(response.status, StatusCode::NOT_FOUND, "{}", response.body);
    assert_eq!(response.json["error"], "not_found");
    assert_eq!(
        project_count(&schema.pool).await,
        1,
        "a non-owner must not delete the project"
    );

    schema.cleanup().await;
}

#[tokio::test]
async fn project_endpoints_require_authentication() {
    let Some((schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping project authentication test");
        return;
    };

    let (_, token) = register_and_login(&app, "alice@example.com", "Alice").await;
    let project_id = create_project(&app, &token, "Alpha").await;
    let uri = format!("/projects/{project_id}");

    let cases: [(&str, String, Option<serde_json::Value>); 5] = [
        (
            "POST",
            "/projects".to_owned(),
            Some(json!({ "name": "Anon" })),
        ),
        ("GET", "/projects".to_owned(), None),
        ("GET", uri.clone(), None),
        ("PATCH", uri.clone(), Some(json!({ "name": "Anon" }))),
        ("DELETE", uri.clone(), None),
    ];

    for (method, uri, body) in cases {
        let response = send(&app, method, &uri, body, None).await;
        assert_eq!(
            response.status,
            StatusCode::UNAUTHORIZED,
            "{method} {uri} must require authentication: {}",
            response.body
        );
        assert_eq!(response.json["error"], "unauthorized");
        assert_eq!(
            response.headers.get("www-authenticate"),
            Some(&"Bearer".parse().expect("static header value"))
        );
    }

    let invalid = send(&app, "GET", "/projects", None, Some("not-a-real-token")).await;
    assert_eq!(invalid.status, StatusCode::UNAUTHORIZED, "{}", invalid.body);
    assert_eq!(invalid.json["error"], "unauthorized");

    assert_eq!(
        project_count(&schema.pool).await,
        1,
        "unauthenticated requests must not create projects"
    );

    schema.cleanup().await;
}

#[tokio::test]
async fn project_schema_enforces_its_invariants() {
    let Some((schema, app)) = setup().await else {
        eprintln!("DATABASE_URL is not set; skipping project schema test");
        return;
    };

    let (user_id, _) = register_and_login(&app, "alice@example.com", "Alice").await;

    // An owner must exist: the foreign key rejects unknown users.
    assert_sqlstate(
        insert_project_raw(&schema.pool, "Valid name", None, user_id + 999)
            .await
            .unwrap_err(),
        "23503",
    );

    // A blank name is rejected by the database, not only by the handler.
    assert_sqlstate(
        insert_project_raw(&schema.pool, "   ", None, user_id)
            .await
            .unwrap_err(),
        "23514",
    );

    // The description cap is enforced in the database too.
    let overlong_description = "a".repeat(server::projects::MAX_PROJECT_DESCRIPTION_LENGTH + 1);
    assert_sqlstate(
        insert_project_raw(
            &schema.pool,
            "Valid name",
            Some(&overlong_description),
            user_id,
        )
        .await
        .unwrap_err(),
        "23514",
    );

    let project_id = insert_project_raw(&schema.pool, "Valid name", None, user_id)
        .await
        .expect("insert a valid project");

    // Deleting an owner with projects is restricted instead of destroying them.
    assert_sqlstate(
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&schema.pool)
            .await
            .unwrap_err(),
        "23503",
    );

    sqlx::query("DELETE FROM projects WHERE id = $1")
        .bind(project_id)
        .execute(&schema.pool)
        .await
        .expect("delete the project");
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&schema.pool)
        .await
        .expect("deleting the user must succeed once its projects are gone");

    // The down migration drops only the projects table.
    let projects_table_before: Option<String> =
        sqlx::query_scalar("SELECT to_regclass('projects')::text")
            .fetch_one(&schema.pool)
            .await
            .expect("look up the projects table before the down migration");
    assert!(
        projects_table_before.is_some(),
        "the projects table must exist before the down migration"
    );

    let migrations = up_migration_files();
    let projects_index = migrations
        .iter()
        .position(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains("create_projects"))
        })
        .expect("the create_projects migration must exist");

    // Roll back the migrations created after `create_projects` first, newest
    // first: a migration can only be undone once nothing it created is still
    // referenced. For example `project_members` has a foreign key to
    // `projects`, so its down script must run before the projects one.
    for path in migrations[projects_index + 1..].iter().rev() {
        apply_sql_file(&schema.pool, &down_migration_file(path)).await;
    }

    apply_sql_file(
        &schema.pool,
        &down_migration_file(&migrations[projects_index]),
    )
    .await;

    let projects_table: Option<String> = sqlx::query_scalar("SELECT to_regclass('projects')::text")
        .fetch_one(&schema.pool)
        .await
        .expect("look up the projects table");
    assert_eq!(
        projects_table, None,
        "the down migration must drop the projects table"
    );
    let users_table: Option<String> = sqlx::query_scalar("SELECT to_regclass('users')::text")
        .fetch_one(&schema.pool)
        .await
        .expect("look up the users table");
    assert!(
        users_table.is_some(),
        "the down migration must not touch users"
    );

    schema.cleanup().await;
}
