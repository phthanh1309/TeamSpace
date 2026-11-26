//! Inline feedback banners.

use dioxus::prelude::*;

/// The tone of an alert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertKind {
    Error,
    Success,
}

impl AlertKind {
    fn class(self) -> &'static str {
        match self {
            Self::Error => "alert alert-error",
            Self::Success => "alert alert-success",
        }
    }
}

/// A short message about the outcome of an action.
#[component]
pub fn Alert(kind: AlertKind, message: String) -> Element {
    let class = kind.class();

    rsx! {
        div { class: "{class}", role: "alert", "{message}" }
    }
}
