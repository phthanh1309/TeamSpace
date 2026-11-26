//! Project list with inline project creation.

use dioxus::prelude::*;

use crate::api::{self, ApiError, Project};
use crate::components::{
    Alert, AlertKind, Button, ButtonVariant, EmptyState, PageHeader, Spinner, TextAreaField,
    TextField,
};
use crate::format;
use crate::routes::Route;
use crate::state::AppState;

/// `GET /projects`, plus the create flow.
#[component]
pub fn DashboardPage() -> Element {
    let state = use_context::<AppState>();
    let mut projects = use_signal(|| None::<Vec<Project>>);
    let mut load_error = use_signal(|| None::<ApiError>);
    let mut reload = use_signal(|| 0u32);
    let mut creating = use_signal(|| false);
    let mut notice = use_signal(|| None::<String>);

    // Refetches whenever the session changes or a mutation asks for a reload.
    use_effect(move || {
        let _ = reload();
        let Some(token) = state.token() else {
            projects.set(None);
            return;
        };

        projects.set(None);
        load_error.set(None);

        spawn(async move {
            match api::list_projects(&token).await {
                Ok(list) => projects.set(Some(list)),
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

    let greeting = state
        .user()
        .map(|user| format!("Hi, {}.", user.display_name))
        .unwrap_or_else(|| "Hi.".to_owned());

    let content = match projects() {
        None => rsx! {
            Spinner { label: "Loading your projects…".to_string() }
        },
        Some(list) if list.is_empty() => rsx! {
            EmptyState {
                title: "No projects yet".to_string(),
                body: "Create your first project. Tasks, discussion and deliverables will live here as the workspace grows.".to_string(),
                Button {
                    onclick: move |_| creating.set(true),
                    "Create a project"
                }
            }
        },
        Some(list) => rsx! {
            div { class: "project-grid",
                for project in list {
                    ProjectCard {
                        key: "{project.id}",
                        project: project.clone(),
                        on_open: move |id| state.navigate(Route::Project { id }),
                    }
                }
            }
        },
    };

    rsx! {
        div { class: "page",
            PageHeader {
                title: "Your projects".to_string(),
                subtitle: Some(greeting),
                Button {
                    onclick: move |_| {
                        notice.set(None);
                        creating.set(true);
                    },
                    "New project"
                }
            }

            if let Some(message) = notice() {
                Alert { kind: AlertKind::Success, message }
            }

            if let Some(error) = load_error() {
                Alert { kind: AlertKind::Error, message: error.user_message() }
            }

            if creating() {
                CreateProjectForm {
                    on_created: move |project: Project| {
                        creating.set(false);
                        notice.set(Some(format!("“{}” was created.", project.name)));
                        reload.set(reload() + 1);
                    },
                    on_cancel: move |_| creating.set(false),
                }
            }

            {content}
        }
    }
}

#[component]
fn ProjectCard(project: Project, on_open: EventHandler<i64>) -> Element {
    let initial = project
        .name
        .chars()
        .next()
        .map(|character| character.to_uppercase().to_string())
        .unwrap_or_default();
    let description = project
        .description
        .clone()
        .unwrap_or_else(|| "No description yet.".to_owned());
    let created = format::short_date(&project.created_at);
    let id = project.id;

    rsx! {
        article { class: "project-card",
            div { class: "project-card-top",
                span { class: "project-initial", "{initial}" }
                span { class: "project-created", "Created {created}" }
            }
            h2 { class: "project-name", "{project.name}" }
            p { class: "project-description", "{description}" }
            button {
                class: "project-open",
                r#type: "button",
                onclick: move |_| on_open.call(id),
                "Open project"
                span { class: "arrow", "→" }
            }
        }
    }
}

#[component]
fn CreateProjectForm(on_created: EventHandler<Project>, on_cancel: EventHandler<()>) -> Element {
    let state = use_context::<AppState>();
    let mut name = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut name_error = use_signal(|| None::<String>);
    let mut form_error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);

    let mut submit = move || {
        if submitting() {
            return;
        }

        let name_value = name().trim().to_owned();
        if name_value.is_empty() {
            name_error.set(Some("Give the project a name.".to_owned()));
            return;
        }
        if name_value.chars().count() > 120 {
            name_error.set(Some("Keep the name to 120 characters or fewer.".to_owned()));
            return;
        }

        name_error.set(None);
        form_error.set(None);
        submitting.set(true);

        let description_value = description().trim().to_owned();

        spawn(async move {
            let Some(token) = state.token() else {
                submitting.set(false);
                return;
            };
            let description_arg = if description_value.is_empty() {
                None
            } else {
                Some(description_value.as_str())
            };

            match api::create_project(&token, &name_value, description_arg).await {
                Ok(project) => on_created.call(project),
                Err(error) => {
                    if error.is_unauthorized() {
                        state.sign_out();
                        state.navigate(Route::Login);
                    }
                    form_error.set(Some(error.user_message()));
                }
            }
            submitting.set(false);
        });
    };

    rsx! {
        section { class: "create-panel",
            h2 { class: "section-title", "New project" }
            p { class: "section-hint", "Start with a clear name. You can add details now or later." }
            if let Some(message) = form_error() {
                Alert { kind: AlertKind::Error, message }
            }
            TextField {
                label: "Project name".to_string(),
                value: name(),
                oninput: move |value| name.set(value),
                placeholder: "Capstone research project".to_string(),
                error: name_error(),
            }
            TextAreaField {
                label: "Description (optional)".to_string(),
                value: description(),
                oninput: move |value| description.set(value),
                placeholder: "What is this project about?".to_string(),
            }
            div { class: "form-actions",
                Button {
                    loading: submitting(),
                    onclick: move |_| submit(),
                    if submitting() { "Creating…" } else { "Create project" }
                }
                Button {
                    variant: ButtonVariant::Ghost,
                    onclick: move |_| on_cancel.call(()),
                    "Cancel"
                }
            }
        }
    }
}
