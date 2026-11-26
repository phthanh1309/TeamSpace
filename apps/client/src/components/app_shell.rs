//! Signed-in application shell: sidebar, identity and content area.

use dioxus::prelude::*;

use crate::api;
use crate::components::{Button, ButtonVariant};
use crate::routes::Route;
use crate::state::AppState;

/// Wraps every authenticated screen.
#[component]
pub fn AppShell(children: Element) -> Element {
    let state = use_context::<AppState>();
    let (display_name, email) = match state.user() {
        Some(user) => (user.display_name, user.email),
        None => (String::new(), String::new()),
    };
    let initial = display_name
        .chars()
        .next()
        .map(|character| character.to_uppercase().to_string())
        .unwrap_or_else(|| "?".to_owned());

    rsx! {
        div { class: "app-shell",
            aside { class: "sidebar",
                div { class: "brand",
                    span { class: "brand-mark", "TS" }
                    span { class: "brand-name", "TeamSpace" }
                }
                nav { class: "nav",
                    span { class: "nav-label", "Workspace" }
                    div { class: "nav-item nav-item-active",
                        span { class: "nav-dot" }
                        "Projects"
                    }
                    p { class: "nav-note", "Tasks, discussion and deliverables arrive next." }
                }
                div { class: "sidebar-footer",
                    div { class: "user-card",
                        span { class: "avatar", "{initial}" }
                        div { class: "user-meta",
                            strong { "{display_name}" }
                            span { "{email}" }
                        }
                    }
                    Button {
                        variant: ButtonVariant::Ghost,
                        onclick: move |_| {
                            spawn(async move {
                                if let Some(token) = state.token() {
                                    let _ = api::logout(&token).await;
                                }
                                state.sign_out();
                                state.navigate(Route::Login);
                            });
                        },
                        "Sign out"
                    }
                }
            }
            main { class: "app-main", {children} }
        }
    }
}
