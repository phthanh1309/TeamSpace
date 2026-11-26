//! One component per screen.

pub mod dashboard;
pub mod login;
pub mod project_detail;
pub mod register;

pub use dashboard::DashboardPage;
pub use login::LoginPage;
pub use project_detail::ProjectDetailPage;
pub use register::RegisterPage;
