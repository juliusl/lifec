mod web_app;
pub use web_app::AppHost;
pub use web_app::WebApp;

mod static_files;
pub use static_files::StaticFiles;

mod route_plugin;
pub use route_plugin::RoutePlugin;

#[cfg(feature = "v2")]
mod engine_server;

#[cfg(feature = "v2")]
pub mod v2 {
    pub use super::engine_server::host_engine;
    pub use super::engine_server::PoemExt;
    pub use super::engine_server::remote_plugin::HyperExt;
    pub use super::engine_server::remote_plugin::RemoteOperation;
}