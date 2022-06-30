use specs::Component;
use crate::plugins::*;
use specs::storage::DenseVecStorage;

use super::ThunkContext;

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

    fn call_with_context(context: &mut ThunkContext) -> Option<JoinHandle<ThunkContext>> {
        context.clone().task(|| {
            let mut tc = context.clone();
            async {
                if let Some(file_block) = tc.block.get_block("file") {
                    if let Some(file_dst) = file_block.find_text("file_dst") {
                        if let Some(content) = file_block.find_binary("content") {
                            match tokio::fs::write(&file_dst, content).await {
                                Ok(_) => {
                                    tc.update_status_only(format!("# wrote file to {}", &file_dst))
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
                } else {
                    tc.error(|a| {
                        a.add_text_attr("error", "missing file block");
                    });
                }

                Some(tc)
            }
        })
    }
}
