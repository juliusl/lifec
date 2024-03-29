use std::{path::PathBuf};

use specs::Component;
use tokio::fs;
use specs::storage::DenseVecStorage;

use crate::plugins::{Plugin, BlockContext};

use super::{ThunkContext, CancelToken, OpenFile};

#[derive(Component, Default)]
#[storage(DenseVecStorage)]
pub struct OpenDir;

impl Plugin<ThunkContext> for OpenDir {
    fn symbol() -> &'static str {
        "open_dir"
    }

    fn description() -> &'static str {
        "Open the contents of a directory."
    }

    fn call_with_context(
        context: &mut ThunkContext,
    ) -> Option<(tokio::task::JoinHandle<ThunkContext>, CancelToken)> {
        context.clone().task(|_| {
            let mut tc = context.clone();

            async move {
                if let Some(file_dir) = tc.as_ref().find_text("file_dir") {
                    tc.update_status_only("file directory found").await;

                    tc.block.block_name = file_dir.to_string();

                    let path_buf = PathBuf::from(file_dir);

                    // Add files to project
                    if let Some(mut read_dir) = fs::read_dir(path_buf).await.ok() {
                        let mut progress = 0.0;
                        loop {
                            match read_dir.next_entry().await {
                                Ok(dir_entry) => {
                                    progress += 0.01;
                                    tc.update_progress("got next entry", progress).await;
                                    match dir_entry {
                                        Some(entry) => {
                                            progress += 0.01;
                                            tc.update_progress(
                                                format!("found entry {:?}", entry),
                                                progress,
                                            )
                                            .await;
                                            let path_buf = entry.path();
                                            let file_src = path_buf.to_str().unwrap_or_default();
                                            let mut work_file = tc.clone();
                                            work_file.as_mut().with_text("file_src", file_src);
                                            if let Some((handle, ..)) =
                                                OpenFile::call_with_context(&mut work_file)
                                            {
                                                progress += 0.01;
                                                tc.update_progress("open file task started", progress).await;
                                                if let Some(result) = handle.await.ok() {
                                                    progress += 0.01;
                                                    tc.update_progress(
                                                        format!(
                                                            "open file task completing, merging, {}",
                                                            result.as_ref().entity()
                                                        ),
                                                        progress,
                                                    )
                                                    .await;

                                                    let file_block =
                                                        result.as_ref().find_imported_graph(
                                                            tc.as_ref().entity() + 1,
                                                        );

                                                    if let Some(imported) = file_block {
                                                        let mut block_context =
                                                            BlockContext::from(imported);

                                                        block_context.update_block(
                                                            "file",
                                                            |file| {
                                                                file.with_text(
                                                                    "file_src", &file_src,
                                                                );
                                                            },
                                                        );

                                                        if let Some(transpiled) =
                                                            block_context.transpile().ok()
                                                        {
                                                            progress += 0.01;
                                                            tc.update_progress(
                                                                format!(
                                                                    "Transpiled {} \n{}",
                                                                    block_context.block_name,
                                                                    transpiled
                                                                ),
                                                                progress,
                                                            )
                                                            .await;

                                                            // transpiles the content into a message
                                                            tc.as_mut().add_message(
                                                                block_context
                                                                    .block_name
                                                                    .trim_start_matches("#"),
                                                                "file",
                                                                transpiled,
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        
                                        }
                                        None => break,
                                    }
                                }
                                Err(_) => break,
                            }
                            progress += 0.01;
                        }
                    }
                }
                Some(tc)
            }
        })
    }
}
