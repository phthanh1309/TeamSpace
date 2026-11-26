//! Friendly placeholder for a list with no items.

use dioxus::prelude::*;

/// A quiet panel explaining why the screen is empty and what to do next.
#[component]
pub fn EmptyState(title: String, body: String, children: Element) -> Element {
    rsx! {
        div { class: "empty-state",
            span { class: "empty-mark", "◇" }
            h2 { "{title}" }
            p { class: "empty-body", "{body}" }
            div { class: "empty-actions", {children} }
        }
    }
}
