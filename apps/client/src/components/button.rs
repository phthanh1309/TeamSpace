//! Buttons with a small, fixed set of visual variants.

use dioxus::prelude::*;

/// Visual weight of a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Ghost,
    Danger,
}

impl ButtonVariant {
    fn class(self) -> &'static str {
        match self {
            Self::Primary => "btn btn-primary",
            Self::Secondary => "btn btn-secondary",
            Self::Ghost => "btn btn-ghost",
            Self::Danger => "btn btn-danger",
        }
    }
}

/// A button that can show a pending state while a request runs.
#[component]
pub fn Button(
    #[props(default)] variant: ButtonVariant,
    #[props(default)] loading: bool,
    #[props(default)] disabled: bool,
    onclick: EventHandler<MouseEvent>,
    children: Element,
) -> Element {
    let is_disabled = disabled || loading;
    let class = variant.class();

    rsx! {
        button {
            class: "{class}",
            r#type: "button",
            disabled: is_disabled,
            onclick: move |event| onclick.call(event),
            if loading {
                span { class: "btn-spinner" }
            }
            {children}
        }
    }
}
