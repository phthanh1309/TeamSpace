//! Project detail with inline editing and a confirmation step before deleting.

use dioxus::prelude::*;

use crate::api::{self, ApiError, Project};
use crate::components::{
    Alert, AlertKind, Button, ButtonVariant, EmptyState, Spinner, TextAreaField, TextField,
};
use crate::format;
use crate::routes::Route;
use crate::state::AppState;

/// `GET /projects/{id}`, `PATCH /projects/{id}`, `DELETE /projects/{id}`.
#[component]
pub fn ProjectDetailPage(project_id: i64) -> Element {
    let state = use_context::<AppState>();
    let mut project = use_signal(|| None::<Project>);
    let mut load_error = use_signal(|| None::<ApiError>);
    let mut editing = use_signal(|| false);
    let mut confirming_delete = use_signal(|| false);
    let mut name = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut name_error = use_signal(|| None::<String>);
    let mut form_error = use_signal(|| None::<String>);
    let mut notice = use_signal(|| None::<String>);
    let mut saving = use_signal(|| false);
    let mut deleting = use_signal(|| false);
    let mut delete_error = use_signal(|| None::<String>);

    use_effect(move || {
        let Some(token) = state.token() else {
            project.set(None);
            return;
        };

        project.set(None);
        load_error.set(None);

        spawn(async move {
            match api::get_project(&token, project_id).await {
                Ok(found) => {
                    name.set(found.name.clone());
                    description.set(found.description.clone().unwrap_or_default());
                    project.set(Some(found));
                }
                Err(error) => {
                    if error.is_unauthorized() {
                        state.sign_out();
                        state.navigate(Route::Login);
                    }
                    load_error.set(Some(error));
                }
            }
        });
    });

    let mut save = move || {
        if saving() {
            return;
        }

        let name_value = name().trim().to_owned();
        if name_value.is_empty() {
            name_error.set(Some("The project needs a name.".to_owned()));
            return;
        }
        if name_value.chars().count() > 120 {
            name_error.set(Some("Keep the name to 120 characters or fewer.".to_owned()));
            return;
        }

        name_error.set(None);
        form_error.set(None);
        saving.set(true);

        let description_value = description().trim().to_owned();

        spawn(async move {
            let Some(token) = state.token() else {
                saving.set(false);
                return;
            };
            let description_arg = if description_value.is_empty() {
                None
            } else {
                Some(description_value.as_str())
            };

            match api::update_project(&token, project_id, &name_value, description_arg).await {
                Ok(updated) => {
                    name.set(updated.name.clone());
                    description.set(updated.description.clone().unwrap_or_default());
                    project.set(Some(updated));
                    editing.set(false);
                    notice.set(Some("Changes saved.".to_owned()));
                }
                Err(error) => {
                    if error.is_unauthorized() {
                        state.sign_out();
                        state.navigate(Route::Login);
                    }
                    form_error.set(Some(error.user_message()));
                }
            }
            saving.set(false);
        });
    };

    let mut delete = move || {
        if deleting() {
            return;
        }

        delete_error.set(None);
        deleting.set(true);

        spawn(async move {
            let Some(token) = state.token() else {
                deleting.set(false);
                return;
            };

            match api::delete_project(&token, project_id).await {
                Ok(()) => state.navigate(Route::Dashboard),
                Err(error) => {
                    if error.is_unauthorized() {
                        state.sign_out();
                        state.navigate(Route::Login);
                    }
                    delete_error.set(Some(error.user_message()));
                }
            }
            deleting.set(false);
        });
    };

    let body = match (project(), load_error()) {
        (_, Some(error)) if error.is_not_found() => rsx! {
            EmptyState {
                title: "Project not found".to_string(),
                body: "It may have been deleted, or it belongs to another user.".to_string(),
                Button {
                    onclick: move |_| state.navigate(Route::Dashboard),
                    "Back to projects"
                }
            }
        },
        (_, Some(error)) => rsx! {
            Alert { kind: AlertKind::Error, message: error.user_message() }
        },
        (None, None) => rsx! {
            Spinner { label: "Loading project…".to_string() }
        },
        (Some(found), None) => rsx! {
            header { class: "detail-head",
                div { class: "detail-head-text",
                    span { class: "eyebrow", "Project" }
                    h1 { class: "detail-title", "{found.name}" }
                    p { class: "detail-meta",
                        "Created {format::short_date(&found.created_at)} · Updated {format::short_date(&found.updated_at)}"
                    }
                }
                div { class: "detail-actions",
                    Button {
                        variant: ButtonVariant::Secondary,
                        onclick: move |_| {
                            notice.set(None);
                            editing.set(!editing());
                        },
                        if editing() { "Cancel edit" } else { "Edit project" }
                    }
                    Button {
                        variant: ButtonVariant::Danger,
                        onclick: move |_| confirming_delete.set(true),
                        "Delete"
                    }
                }
            }

            if let Some(message) = notice() {
                Alert { kind: AlertKind::Success, message }
            }

            section { class: "detail-card",
                h2 { class: "section-title", "About this project" }
                if let Some(description) = found.description.clone() {
                    p { class: "detail-description", "{description}" }
                } else {
                    p { class: "detail-description detail-description-empty",
                        "No description yet. Add one so your team knows what this project is about."
                    }
                }
            }

            if editing() {
                section { class: "detail-card",
                    h2 { class: "section-title", "Edit project" }
                    if let Some(message) = form_error() {
                        Alert { kind: AlertKind::Error, message }
                    }
                    TextField {
                        label: "Project name".to_string(),
                        value: name(),
                        oninput: move |value| name.set(value),
                        error: name_error(),
                    }
                    TextAreaField {
                        label: "Description".to_string(),
                        value: description(),
                        oninput: move |value| description.set(value),
                        hint: Some("Leave this blank to remove the description.".to_string()),
                    }
                    div { class: "form-actions",
                        Button {
                            loading: saving(),
                            onclick: move |_| save(),
                            if saving() { "Saving…" } else { "Save changes" }
                        }
                        Button {
                            variant: ButtonVariant::Ghost,
                            onclick: move |_| editing.set(false),
                            "Cancel"
                        }
                    }
                }
            }

            if confirming_delete() {
                section { class: "confirm-box",
                    h2 { class: "section-title", "Delete this project?" }
                    p { class: "section-hint",
                        "This permanently removes the project and everything inside it. This cannot be undone."
                    }
                    if let Some(message) = delete_error() {
                        Alert { kind: AlertKind::Error, message }
                    }
                    div { class: "form-actions",
                        Button {
                            variant: ButtonVariant::Danger,
                            loading: deleting(),
                            onclick: move |_| delete(),
                            if deleting() { "Deleting…" } else { "Yes, delete project" }
                        }
                        Button {
                            variant: ButtonVariant::Ghost,
                            onclick: move |_| confirming_delete.set(false),
                            "Keep project"
                        }
                    }
                }
            }
        },
    };

    rsx! {
        div { class: "page",
            button {
                class: "back-link",
                r#type: "button",
                onclick: move |_| state.navigate(Route::Dashboard),
                "← All projects"
            }
            {body}
        }
    }
}
