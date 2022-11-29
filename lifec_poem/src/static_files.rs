use std::path::PathBuf;

use crate::{AppHost, WebApp};
use lifec::prelude::*;
use poem::{endpoint::StaticFilesEndpoint, Route};

/// Static files plugin that starts a server host on text_attribute `address`
/// and serving files from `work_dir`. URL will be formatted as {address}/{block_name}/index.html.
/// If index_html is set, then {address}/{block_name} will direct to that file.
#[derive(Default, Clone, Component)]
#[storage(DenseVecStorage)]
pub struct StaticFiles(
    /// work_dir
    PathBuf,
    /// api_prefix
    String,
    // index_html
    Option<String>,
);

impl WebApp for StaticFiles {
    fn create(context: &mut ThunkContext) -> Self {
        let block_name = context
            .state()
            .find_symbol("api_prefix")
            .expect("should have an api prefix");

        if let Some(work_dir) = context.work_dir() {
            if let Some(index_html) = context.state().find_text("index_html") {
                Self(work_dir, block_name, Some(index_html))
            } else {
                Self(work_dir, block_name, None)
            }
        } else {
            Self(PathBuf::from(""), block_name, None)
        }
    }

    fn routes(&mut self) -> Route {
        let Self(work_dir, block_name, index_html) = self;

        let path_prefix = format!("/{block_name}");
        event!(Level::DEBUG, "Hosting, {path_prefix}");
        Route::new().nest(&path_prefix, {
            let mut static_files = StaticFilesEndpoint::new(&work_dir);

            if let Some(index_html) = index_html {
                static_files = static_files.index_file(index_html.to_string());
            }

            static_files
        })
    }
}

impl Plugin for StaticFiles {
    fn symbol() -> &'static str {
        "static_files"
    }

    fn description() -> &'static str {
        "Starts a static file server host for file directory specified by `work_dir`"
    }

    fn call(context: &mut ThunkContext) -> Option<AsyncContext> {
        AppHost::<StaticFiles>::call(context)
    }
}

impl BlockObject for StaticFiles {
    fn query(&self) -> BlockProperties {
        BlockProperties::default()
            .require("app_host")
            .optional("shutdown_timeout_ms")
            .optional("tls_key")
            .optional("tls_crt")
            .require("static_files")
            .require("api_prefix")
            .optional("work_dir")
            .optional("index_html")
    }

    fn parser(&self) -> Option<CustomAttribute> {
        Some(StaticFiles::as_custom_attr())
    }
}
