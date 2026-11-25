//! Shared PostgreSQL harness for integration tests.
//!
//! Every test runs in its own throwaway schema, applies the real migrations to
//! it and drops it afterwards, so the `public` schema is never touched. When
//! `DATABASE_URL` is not set the individual tests report a skip.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::postgres::PgPoolOptions;
use sqlx::{AssertSqlSafe, Connection, Executor, PgConnection, PgPool};

const MIGRATIONS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../migrations");

/// Disambiguates schemas created by tests running in parallel in one process.
static SCHEMA_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn database_url() -> Option<String> {
    dotenvy::dotenv().ok();
    std::env::var("DATABASE_URL").ok()
}

/// A throwaway schema so tests never modify application data.
pub struct TestSchema {
    name: String,
    admin: PgConnection,
    pub pool: PgPool,
}

impl TestSchema {
    pub async fn new(url: &str) -> Self {
        let name = format!(
            "teamspace_test_{}_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock is before the UNIX epoch")
                .as_nanos(),
            SCHEMA_COUNTER.fetch_add(1, Ordering::Relaxed)
        );

        let mut admin = PgConnection::connect(url)
            .await
            .expect("connect to PostgreSQL for test schema setup");

        // The schema name is generated above from a process id and a timestamp,
        // so it contains no untrusted input.
        sqlx::raw_sql(AssertSqlSafe(format!("CREATE SCHEMA \"{name}\"")))
            .execute(&mut admin)
            .await
            .expect("create the temporary test schema");

        let search_path = format!("SET search_path TO \"{name}\"");
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .after_connect(move |connection, _metadata| {
                let search_path = search_path.clone();
                Box::pin(async move {
                    connection
                        .execute(sqlx::raw_sql(AssertSqlSafe(search_path)))
                        .await?;
                    Ok(())
                })
            })
            .connect(url)
            .await
            .expect("connect to the temporary test schema");

        Self { name, admin, pool }
    }

    pub async fn cleanup(mut self) {
        self.pool.close().await;
        let _ = sqlx::raw_sql(AssertSqlSafe(format!(
            "DROP SCHEMA IF EXISTS \"{}\" CASCADE",
            self.name
        )))
        .execute(&mut self.admin)
        .await;
    }
}

pub fn up_migration_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(MIGRATIONS_DIR)
        .unwrap_or_else(|error| panic!("read migrations directory {MIGRATIONS_DIR}: {error}"))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.to_string_lossy().ends_with(".up.sql"))
        .collect();
    files.sort();
    files
}

pub fn down_migration_file(up: &Path) -> PathBuf {
    let file_name = up
        .file_name()
        .expect("migration file has a name")
        .to_string_lossy();
    up.with_file_name(file_name.replace(".up.sql", ".down.sql"))
}

pub async fn apply_sql_file(pool: &PgPool, path: &Path) {
    let sql = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    sqlx::raw_sql(AssertSqlSafe(sql))
        .execute(pool)
        .await
        .unwrap_or_else(|error| panic!("apply {}: {error}", path.display()));
}

pub async fn apply_all_up_migrations(pool: &PgPool) {
    for path in up_migration_files() {
        apply_sql_file(pool, &path).await;
    }
}
