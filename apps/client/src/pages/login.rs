//! Sign-in screen.

use dioxus::prelude::*;

use crate::api;
use crate::components::{Alert, AlertKind, AuthLayout, Button, TextField};
use crate::routes::Route;
use crate::state::{AppState, Session};

/// `POST /auth/login` followed by `GET /auth/me`.
#[component]
pub fn LoginPage() -> Element {
    let state = use_context::<AppState>();
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut email_error = use_signal(|| None::<String>);
    let mut password_error = use_signal(|| None::<String>);
    let mut form_error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);

    let mut submit = move || {
        if submitting() {
            return;
        }

        let email_value = email().trim().to_owned();
        let password_value = password();
        let email_invalid = email_value.is_empty();
        let password_invalid = password_value.is_empty();

        email_error.set(email_invalid.then(|| "Enter your email address.".to_owned()));
        password_error.set(password_invalid.then(|| "Enter your password.".to_owned()));
        if email_invalid || password_invalid {
            return;
        }

        form_error.set(None);
        submitting.set(true);

        spawn(async move {
            let result = match api::login(&email_value, &password_value).await {
                Ok(login) => match api::me(&login.token).await {
                    Ok(user) => {
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
            title: "Welcome back".to_string(),
            subtitle: "Sign in to pick up where your team left off.".to_string(),
            div { class: "auth-form",
                if let Some(message) = form_error() {
                    Alert { kind: AlertKind::Error, message }
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
                    autocomplete: "current-password".to_string(),
                    error: password_error(),
                }
                Button {
                    loading: submitting(),
                    onclick: move |_| submit(),
                    if submitting() { "Signing in…" } else { "Sign in" }
                }
                p { class: "auth-switch",
                    "New to TeamSpace? "
                    button {
                        class: "link-button",
                        r#type: "button",
                        onclick: move |_| state.navigate(Route::Register),
                        "Create an account"
                    }
                }
            }
        }
    }
}
