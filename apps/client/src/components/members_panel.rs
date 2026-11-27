//! Project members: everyone can read the list, only the owner can manage it.
//!
//! The owner is shown with the `Owner` role but is never removable: the owner
//! lives in `projects.owner_id`, not in the membership table.

use dioxus::prelude::*;

use crate::api::{self, ApiError, Member, MemberRole};
use crate::components::{Alert, AlertKind, Button, ButtonVariant, Spinner, TextField};
use crate::format;
use crate::routes::Route;
use crate::state::AppState;

/// Maximum email length accepted by the backend.
const MAX_EMAIL_LENGTH: usize = 255;

/// Member list with the owner-only add and remove flows.
#[component]
pub fn MembersPanel(project_id: i64, is_owner: bool) -> Element {
    let state = use_context::<AppState>();
    let mut members = use_signal(|| None::<Vec<Member>>);
    let mut load_error = use_signal(|| None::<ApiError>);
    let mut reload = use_signal(|| 0u32);

    let mut adding = use_signal(|| false);
    let mut email = use_signal(String::new);
    let mut email_error = use_signal(|| None::<String>);
    let mut add_error = use_signal(|| None::<String>);
    let mut adding_busy = use_signal(|| false);

    let mut confirming_remove = use_signal(|| None::<i64>);
    let mut removing = use_signal(|| false);
    let mut remove_error = use_signal(|| None::<String>);
    let mut notice = use_signal(|| None::<String>);

    // Refetches on mount and whenever a mutation asks for a reload.
    use_effect(move || {
        let _ = reload();
        let Some(token) = state.token() else {
            members.set(None);
            return;
        };

        load_error.set(None);

        spawn(async move {
            match api::list_members(&token, project_id).await {
                Ok(list) => members.set(Some(list)),
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

    let mut submit = move || {
        if adding_busy() {
            return;
        }

        let email_value = email().trim().to_owned();
        let invalid = !email_value.contains('@') || email_value.chars().count() > MAX_EMAIL_LENGTH;
        email_error.set(invalid.then(|| "Enter the email address they signed up with.".to_owned()));
        if invalid {
            return;
        }

        add_error.set(None);
        adding_busy.set(true);

        spawn(async move {
            let Some(token) = state.token() else {
                adding_busy.set(false);
                return;
            };

            match api::add_member(&token, project_id, &email_value).await {
                Ok(member) => {
                    notice.set(Some(format!(
                        "{} was added to this project.",
                        member.display_name
                    )));
                    email.set(String::new());
                    adding.set(false);
                    reload.set(reload() + 1);
                }
                Err(error) => {
                    if error.is_unauthorized() {
                        state.sign_out();
                        state.navigate(Route::Login);
                    }
                    add_error.set(Some(error.user_message()));
                }
            }
            adding_busy.set(false);
        });
    };

    let remove = EventHandler::new(move |user_id: i64| {
        if removing() {
            return;
        }

        remove_error.set(None);
        removing.set(true);

        spawn(async move {
            let Some(token) = state.token() else {
                removing.set(false);
                return;
            };

            match api::remove_member(&token, project_id, user_id).await {
                Ok(()) => {
                    confirming_remove.set(None);
                    notice.set(Some("Member removed.".to_owned()));
                    reload.set(reload() + 1);
                }
                Err(error) => {
                    if error.is_unauthorized() {
                        state.sign_out();
                        state.navigate(Route::Login);
                    }
                    remove_error.set(Some(error.user_message()));
                }
            }
            removing.set(false);
        });
    });

    let body = if let Some(error) = load_error() {
        rsx! {
            Alert { kind: AlertKind::Error, message: error.user_message() }
            div { class: "form-actions",
                Button {
                    variant: ButtonVariant::Secondary,
                    onclick: move |_| reload.set(reload() + 1),
                    "Try again"
                }
            }
        }
    } else {
        match members() {
            None => rsx! {
                Spinner { label: "Loading members…".to_string() }
            },
            Some(list) if list.is_empty() => rsx! {
                p { class: "section-hint", "No members yet." }
            },
            Some(list) => rsx! {
                div { class: "member-list",
                    for member in list {
                        MemberRow {
                            key: "{member.user_id}",
                            can_remove: is_owner && member.role == MemberRole::Member,
                            confirming: confirming_remove() == Some(member.user_id),
                            busy: removing(),
                            member: member.clone(),
                            on_ask_remove: move |user_id| {
                                remove_error.set(None);
                                confirming_remove.set(Some(user_id));
                            },
                            on_cancel: move |_| confirming_remove.set(None),
                            on_confirm: remove,
                        }
                    }
                }
            },
        }
    };

    rsx! {
        section { class: "detail-card members-card",
            div { class: "members-head",
                div { class: "members-head-text",
                    h2 { class: "section-title", "Members" }
                    p { class: "section-hint",
                        if is_owner {
                            "Add teammates who already have a TeamSpace account."
                        } else {
                            "People with access to this project."
                        }
                    }
                }
                if is_owner {
                    Button {
                        variant: ButtonVariant::Secondary,
                        onclick: move |_| {
                            add_error.set(None);
                            email_error.set(None);
                            adding.set(!adding());
                        },
                        if adding() { "Close" } else { "Add member" }
                    }
                }
            }

            if let Some(message) = notice() {
                Alert { kind: AlertKind::Success, message }
            }
            if let Some(message) = add_error() {
                Alert { kind: AlertKind::Error, message }
            }
            if let Some(message) = remove_error() {
                Alert { kind: AlertKind::Error, message }
            }

            if adding() && is_owner {
                div { class: "add-member-form",
                    TextField {
                        label: "Email address".to_string(),
                        value: email(),
                        oninput: move |value| email.set(value),
                        input_type: "email".to_string(),
                        placeholder: "teammate@university.edu".to_string(),
                        error: email_error(),
                        hint: Some("They must have registered already.".to_string()),
                    }
                    div { class: "form-actions",
                        Button {
                            loading: adding_busy(),
                            onclick: move |_| submit(),
                            if adding_busy() { "Adding…" } else { "Add member" }
                        }
                        Button {
                            variant: ButtonVariant::Ghost,
                            onclick: move |_| {
                                add_error.set(None);
                                email_error.set(None);
                                adding.set(false);
                            },
                            "Cancel"
                        }
                    }
                }
            }

            {body}
        }
    }
}

#[component]
fn MemberRow(
    member: Member,
    can_remove: bool,
    confirming: bool,
    busy: bool,
    on_ask_remove: EventHandler<i64>,
    on_cancel: EventHandler<()>,
    on_confirm: EventHandler<i64>,
) -> Element {
    let initial = member
        .display_name
        .chars()
        .next()
        .map(|character| character.to_uppercase().to_string())
        .unwrap_or_else(|| "?".to_owned());
    let role_class = match member.role {
        MemberRole::Owner => "member-role member-role-owner",
        MemberRole::Member => "member-role member-role-member",
    };
    let user_id = member.user_id;

    rsx! {
        div { class: "member-row",
            span { class: "avatar avatar-light", "{initial}" }
            div { class: "member-meta",
                div { class: "member-line",
                    strong { class: "member-name", "{member.display_name}" }
                    span { class: "{role_class}", "{member.role.label()}" }
                }
                span { class: "member-email", "{member.email}" }
                span { class: "member-joined",
                    "Joined {format::short_date(&member.joined_at)}"
                }
            }
            if can_remove {
                if confirming {
                    div { class: "member-confirm",
                        span { class: "member-confirm-label", "Remove?" }
                        Button {
                            variant: ButtonVariant::Danger,
                            loading: busy,
                            onclick: move |_| on_confirm.call(user_id),
                            if busy { "Removing…" } else { "Remove" }
                        }
                        Button {
                            variant: ButtonVariant::Ghost,
                            onclick: move |_| on_cancel.call(()),
                            "Cancel"
                        }
                    }
                } else {
                    button {
                        class: "link-danger",
                        r#type: "button",
                        onclick: move |_| on_ask_remove.call(user_id),
                        "Remove"
                    }
                }
            }
        }
    }
}
