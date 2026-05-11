pub mod api;
pub mod dashboard;
pub mod serve;

#[cfg(test)]
mod api_tests;

pub use api::{AppState, build_router};
pub use dashboard::start_dashboard;
pub use serve::start_static_server;
