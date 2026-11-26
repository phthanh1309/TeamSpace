//! Session and navigation state shared through Dioxus context.
//!
//! The bearer token lives in memory only. It is never written to disk or
//! logged, so closing the app signs the user out. Persisting it is a deliberate
//! later step that depends on the final deployment target.

use dioxus::prelude::*;

use crate::api::User;
use crate::routes::Route;

/// An authenticated user together with the token that proves it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub token: String,
    pub user: User,
}

/// Application-wide state handed to every page through context.
#[derive(Clone, Copy)]
pub struct AppState {
    pub route: Signal<Route>,
    pub session: Signal<Option<Session>>,
}

impl AppState {
    /// Moves to another screen.
    pub fn navigate(&self, route: Route) {
        let mut route_signal = self.route;
        route_signal.set(route);
    }

    /// Stores a successful login.
    pub fn sign_in(&self, session: Session) {
        let mut session_signal = self.session;
        session_signal.set(Some(session));
    }

    /// Forgets the current session. The token itself is dropped with it.
    pub fn sign_out(&self) {
        let mut session_signal = self.session;
        session_signal.set(None);
    }

    /// Returns a copy of the bearer token, if signed in.
    pub fn token(&self) -> Option<String> {
        self.session
            .read()
            .as_ref()
            .map(|session| session.token.clone())
    }

    /// Returns a copy of the signed-in user, if any.
    pub fn user(&self) -> Option<User> {
        self.session
            .read()
            .as_ref()
            .map(|session| session.user.clone())
    }

    pub fn is_authenticated(&self) -> bool {
        self.session.read().is_some()
    }
}
