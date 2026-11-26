//! Application root: owns the session and the current screen.

use dioxus::prelude::*;

use crate::components::AppShell;
use crate::pages::{DashboardPage, LoginPage, ProjectDetailPage, RegisterPage};
use crate::routes::Route;
use crate::state::{AppState, Session};

/// Design tokens and component styles, kept in one asset file.
const CSS: &str = include_str!("../assets/main.css");

#[component]
pub fn App() -> Element {
    let route = use_signal(|| Route::Login);
    let session = use_signal(|| None::<Session>);
    let state = use_context_provider(|| AppState { route, session });

    // Unauthenticated users only ever see the auth screens, and authenticated
    // users never fall back to them, so the two areas cannot be mixed up.
    let content = if state.is_authenticated() {
        match route() {
            Route::Project { id } => rsx! {
                AppShell {
                    ProjectDetailPage { key: "{id}", project_id: id }
                }
            },
            _ => rsx! {
                AppShell { DashboardPage {} }
            },
        }
    } else {
        match route() {
            Route::Register => rsx! { RegisterPage {} },
            _ => rsx! { LoginPage {} },
        }
    };

    rsx! {
        style { dangerous_inner_html: CSS }
        {content}
    }
}
