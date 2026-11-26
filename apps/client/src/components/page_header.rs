//! Page title block with an optional action area.

use dioxus::prelude::*;

/// A page heading, an optional subtitle and right-aligned actions.
#[component]
pub fn PageHeader(
    title: String,
    #[props(default)] subtitle: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        header { class: "page-head",
            div { class: "page-head-text",
                h1 { "{title}" }
                if let Some(subtitle) = subtitle {
                    p { class: "page-subtitle", "{subtitle}" }
                }
            }
            div { class: "page-head-actions", {children} }
        }
    }
}
