use std::path::PathBuf;

use reality::{BlockObject, BlockProperties};
use specs::{Component, DenseVecStorage};

use crate::plugins::{ThunkContext, Plugin};

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
        "Installs a file from the src_dir to work_dir"
    }

    fn call(context: &ThunkContext) -> Option<crate::plugins::AsyncContext> {
        context.clone().query::<Install>().task(|_|{
            let tc = context.clone();
            async {
                let properties = tc.clone().block.properties.expect("there should be properties");
                let file_name = properties
                    .property("install").expect("a file name")
                    .symbol().expect("a symbol value");

                let src_dir = properties
                    .property("src_dir").expect("a src directory")
                    .symbol().expect("a symbol value");

                let work_dir = properties
                    .property("work_dir").expect("a work_dir")
                    .symbol().expect("a symbol value");

                let src = PathBuf::from(src_dir).join(file_name);
                let dst = PathBuf::from(work_dir).join(file_name);
                
                match tokio::fs::copy(src, dst).await {
                    Ok(_) => {
                        Some(tc)
                    },
                    Err(err) => {
                        panic!("Could not copy files {err}");
                    },
                }
            }
        })
    }
}

impl BlockObject for Install {
    fn query(&self) -> reality::BlockProperties {
        BlockProperties::default()
            .require("install")
            .require("work_dir")
            .require("src_dir")
    }

    fn parser(&self) -> Option<reality::CustomAttribute> {
        Some(Install::as_custom_attr())
    }
}