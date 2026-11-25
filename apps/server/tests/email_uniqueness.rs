//! Integration tests for case-insensitive email handling.
//!
//! The tests need a running PostgreSQL instance configured through
//! `DATABASE_URL` (loaded from the workspace `.env` when present). Each test
//! works in its own temporary schema and never touches the `public` schema.
//! When `DATABASE_URL` is not set the tests report a skip instead of failing.

mod support;

use server::users::{create_user, find_user_id_by_email};
use sqlx::PgPool;
use support::{
    TestSchema, apply_all_up_migrations, apply_sql_file, database_url, down_migration_file,
    up_migration_files,
};

fn assert_unique_violation(error: sqlx::Error) {
    let database_error = error
        .as_database_error()
        .unwrap_or_else(|| panic!("expected a database error, got: {error}"));
    assert_eq!(
        database_error.code().as_deref(),
        Some("23505"),
        "expected a unique violation, got: {database_error}"
    );
}

async fn insert_user(pool: &PgPool, email: &str, display_name: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO users (email, password_hash, display_name) VALUES ($1, $2, $3)")
        .bind(email)
        .bind("test-password-hash")
        .bind(display_name)
        .execute(pool)
        .await?;
    Ok(())
}

#[tokio::test]
async fn migration_normalizes_existing_emails_without_losing_rows() {
    let Some(url) = database_url() else {
        eprintln!("DATABASE_URL is not set; skipping PostgreSQL migration test");
        return;
    };

    let schema = TestSchema::new(&url).await;
    let files = up_migration_files();
    let email_migration = files
        .iter()
        .position(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains("users_email_case_insensitive_unique"))
        })
        .expect("the case-insensitive email migration must exist");

    for path in &files[..email_migration] {
        apply_sql_file(&schema.pool, path).await;
    }
    let email_migration = &files[email_migration];

    insert_user(&schema.pool, "  Alice@Example.COM  ", "Alice")
        .await
        .expect("insert a row through the pre-migration schema");
    insert_user(&schema.pool, "bob@example.com", "Bob")
        .await
        .expect("insert a row through the pre-migration schema");

    apply_sql_file(&schema.pool, email_migration).await;

    let emails: Vec<String> = sqlx::query_scalar("SELECT email FROM users ORDER BY id")
        .fetch_all(&schema.pool)
        .await
        .expect("read emails after the migration");
    assert_eq!(
        emails,
        vec!["alice@example.com".to_owned(), "bob@example.com".to_owned()],
        "the migration must keep every row and store the canonical form"
    );

    let legacy_unique_constraints: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_constraint \
         WHERE conrelid = 'users'::regclass \
           AND contype = 'u' \
           AND pg_get_constraintdef(oid) = 'UNIQUE (email)'",
    )
    .fetch_one(&schema.pool)
    .await
    .expect("count unique constraints on email");
    assert_eq!(
        legacy_unique_constraints, 0,
        "the old case-sensitive UNIQUE(email) constraint must be dropped"
    );

    // Verify the index through catalog metadata instead of matching the exact
    // text of `pg_get_indexdef`, which includes the schema name and casts such
    // as `lower((email)::text)`.
    let (
        is_unique,
        key_columns,
        is_expression_index,
        first_key_attribute,
        key_definition,
        references_email,
    ) = sqlx::query_as::<_, (bool, i16, bool, i16, String, bool)>(
        "SELECT \
             i.indisunique, \
             i.indnkeyatts, \
             (i.indexprs IS NOT NULL), \
             i.indkey[0], \
             pg_get_indexdef(i.indexrelid, 1, true), \
             EXISTS ( \
                 SELECT 1 \
                 FROM pg_depend d \
                 JOIN pg_attribute a \
                   ON a.attrelid = i.indrelid \
                  AND a.attnum = d.refobjsubid \
                 WHERE d.classid = 'pg_class'::regclass \
                   AND d.objid = i.indexrelid \
                   AND d.refclassid = 'pg_class'::regclass \
                   AND d.refobjid = i.indrelid \
                   AND a.attname = 'email' \
             ) \
         FROM pg_index i \
         JOIN pg_class idx ON idx.oid = i.indexrelid \
         JOIN pg_class tbl ON tbl.oid = i.indrelid \
         JOIN pg_namespace ns ON ns.oid = idx.relnamespace \
         WHERE ns.nspname = current_schema() \
           AND tbl.relname = 'users' \
           AND idx.relname = 'users_email_lower_key'",
    )
    .fetch_one(&schema.pool)
    .await
    .expect("read the case-insensitive unique index from the catalog");

    assert!(is_unique, "users_email_lower_key must be a unique index");
    assert_eq!(
        key_columns, 1,
        "users_email_lower_key must have exactly one key column"
    );
    assert!(
        is_expression_index,
        "users_email_lower_key must be an expression index"
    );
    assert_eq!(
        first_key_attribute, 0,
        "the index key must be an expression, not a plain column"
    );
    assert!(
        references_email,
        "the index expression must reference the users.email column"
    );

    let normalized_key_definition: String = key_definition
        .to_lowercase()
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    assert!(
        normalized_key_definition.starts_with("lower("),
        "expected a lower(...) expression index key, got: {key_definition}"
    );

    assert_unique_violation(
        insert_user(&schema.pool, "ALICE@example.com", "Alice again")
            .await
            .expect_err("a case variant must be rejected"),
    );

    let down = down_migration_file(email_migration);
    apply_sql_file(&schema.pool, &down).await;
    insert_user(&schema.pool, "ALICE@example.com", "Alice after rollback")
        .await
        .expect("the down migration restores case-sensitive uniqueness");

    schema.cleanup().await;
}

#[tokio::test]
async fn create_user_normalizes_email_and_lookup_is_case_insensitive() {
    let Some(url) = database_url() else {
        eprintln!("DATABASE_URL is not set; skipping PostgreSQL user test");
        return;
    };

    let schema = TestSchema::new(&url).await;
    apply_all_up_migrations(&schema.pool).await;

    let id = create_user(
        &schema.pool,
        "  Alice@Example.COM  ",
        "test-password-hash",
        "Alice",
    )
    .await
    .expect("insert a user with a normal email");

    let stored_email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(&schema.pool)
        .await
        .expect("read the stored email");
    assert_eq!(stored_email, "alice@example.com");

    let found = find_user_id_by_email(&schema.pool, " ALICE@EXAMPLE.COM ")
        .await
        .expect("look up a differently cased email");
    assert_eq!(found, Some(id));

    let duplicate = create_user(
        &schema.pool,
        "aLiCe@example.com",
        "another-password-hash",
        "Alice duplicate",
    )
    .await
    .expect_err("a case variant must not be stored twice");
    assert_unique_violation(duplicate);

    let other_id = create_user(&schema.pool, "bob@example.com", "test-password-hash", "Bob")
        .await
        .expect("a different email must still be accepted");
    assert_ne!(other_id, id);

    schema.cleanup().await;
}
