//! TeamSpace desktop client.
//!
//! The app is a thin Dioxus UI over the existing Axum API. Module layout:
//!
//! - [`api`] — typed HTTP calls and the shared error type.
//! - [`state`] — session plus in-memory routing state shared through context.
//! - [`routes`] — the small set of screens in this milestone.
//! - [`components`] — reusable layout and form primitives.
//! - [`pages`] — one component per screen.

mod api;
mod app;
mod components;
mod format;
mod pages;
mod routes;
mod state;

fn main() {
    dioxus::launch(app::App);
}
