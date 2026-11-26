//! Loading indicator with a short description.

use dioxus::prelude::*;

/// A centred spinner plus a label describing what is loading.
#[component]
pub fn Spinner(label: String) -> Element {
    rsx! {
        div { class: "spinner-wrap",
            span { class: "spinner" }
            span { class: "spinner-label", "{label}" }
        }
    }
}
