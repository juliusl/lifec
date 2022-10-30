use std::path::PathBuf;

pub use crate::prelude::*;

/// Component for installing scripts,
///
/// In summary copies files from a src_dir to a work_dir
///
#[derive(Default, Clone, Debug, Component)]
#[storage(DenseVecStorage)]
pub struct Install;

impl Plugin for Install {
    fn symbol() -> &'static str {
        "install"
    }

    fn description() -> &'static str {
        "Installs a file to the work_dir"
    }

    fn call(context: &ThunkContext) -> Option<crate::plugins::AsyncContext> {
        context.clone().task(|_| {
            let tc = context.clone();
            async {
                let file_name = tc
                    .state()
                    .find_symbol("install")
                    .expect("file name is required for install plugin");
                let src_dir = tc
                    .state()
                    .find_symbol("src_dir")
                    .unwrap_or(String::default());
                    
                let work_dir = tc
                    .work_dir()
                    .expect("work_dir required for install plugin");

                tokio::fs::create_dir_all(&work_dir)
                    .await
                    .expect("should be able to create work directory");

                let src = PathBuf::from(src_dir).join(&file_name);
                let dst = PathBuf::from(work_dir).join(&file_name);

                match tokio::fs::copy(src, dst).await {
                    Ok(_) => Some(tc),
                    Err(err) => {
                        panic!("Could not copy files {err}");
                    }
                }
            }
        })
    }
}

impl BlockObject for Install {
    fn query(&self) -> BlockProperties {
        BlockProperties::default()
            .require("install")
            .require("work_dir")
            .require("src_dir")
    }

    fn parser(&self) -> Option<CustomAttribute> {
        Some(Install::as_custom_attr())
    }
}
