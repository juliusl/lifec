use std::path::PathBuf;

use specs::Component;
use tokio::fs;

use crate::plugins::*;
use specs::storage::DenseVecStorage;

use super::ThunkContext;

#[derive(Component, Default)]
#[storage(DenseVecStorage)]
pub struct OpenFile;

impl Plugin<ThunkContext> for OpenFile {
    fn symbol() -> &'static str {
        "open_file"
    }

    fn description() -> &'static str {
        "Open and reads a file to a string, and then imports to a binary attribute."
    }

    fn call_with_context(context: &mut ThunkContext) -> Option<tokio::task::JoinHandle<ThunkContext>> {
        context.clone().task(||{
            let mut tc = context.clone();
            async {
                if let Some(file_src) = tc.as_ref().find_text("file_src") {
                    tc.update_status_only("file source found").await;

                    let path_buf = PathBuf::from(file_src);

                    if let Some(content) = fs::read_to_string(&path_buf).await.ok() {
                        tc.update_status_only("read content, writing to block").await;
                        if tc.block.add_block(path_buf.file_name().unwrap_or_default().to_str().unwrap_or_default(), |c| {
                            c.add_binary_attr("content", content.as_bytes());
                        }) {
                            tc.update_status_only("added file").await;
                        }
                    }
                }
                Some(tc)
            }
        })
    }
}