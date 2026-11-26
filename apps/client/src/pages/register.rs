//! Account creation screen.
//!
//! A successful registration is followed by an automatic sign-in so the user
//! lands on the dashboard instead of retyping the password.

use dioxus::prelude::*;

use crate::api;
use crate::components::{Alert, AlertKind, AuthLayout, Button, TextField};
use crate::routes::Route;
use crate::state::{AppState, Session};

const MIN_PASSWORD_LENGTH: usize = 8;

#[component]
pub fn RegisterPage() -> Element {
    let state = use_context::<AppState>();
    let mut display_name = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut name_error = use_signal(|| None::<String>);
    let mut email_error = use_signal(|| None::<String>);
    let mut password_error = use_signal(|| None::<String>);
    let mut form_error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);

    let mut submit = move || {
        if submitting() {
            return;
        }

        let name_value = display_name().trim().to_owned();
        let email_value = email().trim().to_owned();
        let password_value = password();

        let name_invalid = name_value.is_empty();
        let email_invalid = !email_value.contains('@');
        let password_invalid = password_value.chars().count() < MIN_PASSWORD_LENGTH;

        name_error.set(name_invalid.then(|| "Enter your name.".to_owned()));
        email_error.set(email_invalid.then(|| "Enter a valid email address.".to_owned()));
        password_error.set(
            password_invalid.then(|| format!("Use at least {MIN_PASSWORD_LENGTH} characters.")),
        );
        if name_invalid || email_invalid || password_invalid {
            return;
        }

        form_error.set(None);
        submitting.set(true);

        spawn(async move {
            let result = match api::register(&email_value, &password_value, &name_value).await {
                Ok(user) => match api::login(&email_value, &password_value).await {
                    Ok(login) => {
                        state.sign_in(Session {
                            token: login.token,
                            user,
                        });
                        state.navigate(Route::Dashboard);
                        Ok(())
                    }
                    Err(error) => Err(error.user_message()),
                },
                Err(error) => Err(error.user_message()),
            };

            if let Err(message) = result {
                form_error.set(Some(message));
            }
            submitting.set(false);
        });
    };

    rsx! {
        AuthLayout {
            title: "Create your account".to_string(),
            subtitle: "Set up a workspace for your next team project.".to_string(),
            div { class: "auth-form",
                if let Some(message) = form_error() {
                    Alert { kind: AlertKind::Error, message }
                }
                TextField {
                    label: "Name".to_string(),
                    value: display_name(),
                    oninput: move |value| display_name.set(value),
                    autocomplete: "name".to_string(),
                    placeholder: "An Nguyen".to_string(),
                    error: name_error(),
                }
                TextField {
                    label: "Email".to_string(),
                    value: email(),
                    oninput: move |value| email.set(value),
                    input_type: "email".to_string(),
                    autocomplete: "email".to_string(),
                    placeholder: "you@university.edu".to_string(),
                    error: email_error(),
                }
                TextField {
                    label: "Password".to_string(),
                    value: password(),
                    oninput: move |value| password.set(value),
                    input_type: "password".to_string(),
                    autocomplete: "new-password".to_string(),
                    hint: Some(format!("At least {MIN_PASSWORD_LENGTH} characters.")),
                    error: password_error(),
                }
                Button {
                    loading: submitting(),
                    onclick: move |_| submit(),
                    if submitting() { "Creating account…" } else { "Create account" }
                }
                p { class: "auth-switch",
                    "Already have an account? "
                    button {
                        class: "link-button",
                        r#type: "button",
                        onclick: move |_| state.navigate(Route::Login),
                        "Sign in"
                    }
                }
            }
        }
    }
}
