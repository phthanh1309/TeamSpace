use axum::{Router, extract::State, http::StatusCode, routing::get};
use sqlx::{PgPool, migrate::Migrator, postgres::PgPoolOptions};
use tokio::net::TcpListener;

// Paths in `migrate!` are relative to this crate's Cargo.toml
// (apps/server), so this resolves to the workspace-level migrations dir.
static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

async fn health() -> &'static str {
    "TeamSpace API is running!"
}

async fn health_db(State(pool): State<PgPool>) -> Result<&'static str, (StatusCode, &'static str)> {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&pool)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database connection failed",
            )
        })?;

    Ok("TeamSpace database connected!")
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to PostgreSQL");

    println!("Connected to PostgreSQL!");

    MIGRATOR
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    println!("Database migrations applied!");

    let app = Router::new()
        .route("/health", get(health))
        .route("/health/db", get(health_db))
        .with_state(pool);

    let listener = TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("Failed to bind address");

    println!("TeamSpace server running at http://127.0.0.1:3000");

    axum::serve(listener, app).await.expect("Server failed");
}
