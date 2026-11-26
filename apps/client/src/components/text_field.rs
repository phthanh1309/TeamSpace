//! Labelled text inputs with inline validation messages.

use dioxus::prelude::*;

/// A single-line labelled input.
#[component]
pub fn TextField(
    label: String,
    value: String,
    oninput: EventHandler<String>,
    #[props(default)] input_type: String,
    #[props(default)] placeholder: String,
    #[props(default)] autocomplete: String,
    #[props(default)] error: Option<String>,
    #[props(default)] hint: Option<String>,
) -> Element {
    let input_type = if input_type.is_empty() {
        "text".to_owned()
    } else {
        input_type
    };
    let class = if error.is_some() {
        "field-input field-input-error"
    } else {
        "field-input"
    };

    rsx! {
        label { class: "field",
            span { class: "field-label", "{label}" }
            input {
                class: "{class}",
                r#type: "{input_type}",
                value: "{value}",
                placeholder: "{placeholder}",
                autocomplete: "{autocomplete}",
                oninput: move |event| oninput.call(event.value()),
            }
            {match (error, hint) {
                (Some(error), _) => rsx! { span { class: "field-error", "{error}" } },
                (None, Some(hint)) => rsx! { span { class: "field-hint", "{hint}" } },
                (None, None) => rsx! {},
            }}
        }
    }
}

/// A labelled multi-line input.
#[component]
pub fn TextAreaField(
    label: String,
    value: String,
    oninput: EventHandler<String>,
    #[props(default)] placeholder: String,
    #[props(default)] error: Option<String>,
    #[props(default)] hint: Option<String>,
) -> Element {
    let class = if error.is_some() {
        "field-input field-textarea field-input-error"
    } else {
        "field-input field-textarea"
    };

    rsx! {
        label { class: "field",
            span { class: "field-label", "{label}" }
            textarea {
                class: "{class}",
                rows: "4",
                value: "{value}",
                placeholder: "{placeholder}",
                oninput: move |event| oninput.call(event.value()),
            }
            {match (error, hint) {
                (Some(error), _) => rsx! { span { class: "field-error", "{error}" } },
                (None, Some(hint)) => rsx! { span { class: "field-hint", "{hint}" } },
                (None, None) => rsx! {},
            }}
        }
    }
}
