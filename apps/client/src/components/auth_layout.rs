//! Two-panel layout for the sign-in and sign-up screens.

use dioxus::prelude::*;

/// Brand panel on the left, form card on the right.
#[component]
pub fn AuthLayout(title: String, subtitle: String, children: Element) -> Element {
    rsx! {
        div { class: "auth-shell",
            aside { class: "auth-brand",
                div { class: "brand",
                    span { class: "brand-mark", "TS" }
                    span { class: "brand-name", "TeamSpace" }
                }
                h1 { class: "auth-brand-title", "From first idea to finished deliverable." }
                p { class: "auth-brand-body",
                    "TeamSpace keeps your student team's projects, tasks and reviews in one calm place."
                }
                ol { class: "workflow-list",
                    li { span { class: "workflow-step", "1" } "Project" }
                    li { span { class: "workflow-step", "2" } "Tasks" }
                    li { span { class: "workflow-step", "3" } "Discussion" }
                    li { span { class: "workflow-step", "4" } "Deliverables" }
                    li { span { class: "workflow-step", "5" } "Review" }
                }
            }
            main { class: "auth-panel",
                div { class: "auth-card",
                    h2 { class: "auth-title", "{title}" }
                    p { class: "auth-subtitle", "{subtitle}" }
                    {children}
                }
            }
        }
    }
}
