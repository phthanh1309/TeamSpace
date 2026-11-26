//! Screens available in this milestone.
//!
//! Navigation is kept in memory instead of a URL router: the desktop target
//! has no address bar and the screen set is small. Adding the Dioxus router
//! later only requires mapping these variants to paths.

/// A screen the user can be on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Login,
    Register,
    Dashboard,
    Project { id: i64 },
}
