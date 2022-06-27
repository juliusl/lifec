use specs::{Component, storage};
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
        "Open and reads a file to a string, and then imports as an attribute."
    }

    fn config(context: &mut ThunkContext) {
        context.as_mut().add_text_attr("file_src", ".runmd");
    }

    fn call_with_context(context: &mut ThunkContext) -> Option<tokio::task::JoinHandle<ThunkContext>> {
        context.clone().task(||{
            let mut tc = context.clone();
            async {
                if let Some(file_src) = tc.as_ref().find_text("file_src") {
                    tc.update_status_only("file source found").await;
                    if let Some(content) = fs::read_to_string(&file_src).await.ok() {
                        tc.update_status_only("read content, writing to block").await;
                        if tc.block.add_block(file_src, |c| {
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