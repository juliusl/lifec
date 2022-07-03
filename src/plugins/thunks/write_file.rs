use std::path::PathBuf;

use crate::plugins::*;
use specs::storage::DenseVecStorage;
use specs::Component;

use super::{CancelToken, ThunkContext};

#[derive(Component, Default)]
#[storage(DenseVecStorage)]
pub struct WriteFile;

impl Plugin<ThunkContext> for WriteFile {
    fn symbol() -> &'static str {
        "write_file"
    }

    fn description() -> &'static str {
        "Writes a file_block to the path specified by file_dst."
    }

    fn call_with_context(
        context: &mut ThunkContext,
    ) -> Option<(JoinHandle<ThunkContext>, CancelToken)> {
        context.clone().task(|_| {
            let mut tc = context.clone();
            async move {
                tc.as_mut().apply("previous");
                for file_block in tc.as_ref().find_blocks("file") {
                    if let Some(work_dir) = tc.as_ref().find_text("work_dir") {
                        if let Some(file_name) = file_block.find_text("file_name") {
                            if let Some(content) = file_block.find_binary("content") {
                                let path = PathBuf::from(&work_dir);
                                tokio::fs::create_dir_all(&work_dir).await.ok();

                                let path = path.join(file_name);

                                match tokio::fs::write(&path, content).await {
                                    Ok(_) => {
                                        tc.update_status_only(format!(
                                            "# wrote file to {:?}",
                                            path
                                        ))
                                        .await;
                                    }
                                    Err(err) => {
                                        let error_message = format!("# error writing file {}", err);
                                        tc.update_status_only(error_message).await;
                                        tc.error(|a| {
                                            a.add_text_attr("error", format!("{}", err));
                                        });
                                    }
                                }
                            } else {
                                tc.error(|a| {
                                    a.add_text_attr("error", "missing content");
                                });
                            }
                        } else {
                            tc.error(|a| {
                                a.add_text_attr("error", "missing file destination");
                            });
                        }
                    }
                }

                Some(tc)
            }
        })
    }
}
