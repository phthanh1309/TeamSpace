//! Library surface for the TeamSpace server.
//!
//! Process startup and HTTP wiring live in `main.rs`; the modules exposed here
//! hold behaviour that integration tests exercise against PostgreSQL.

pub mod users;
