use std::{path::PathBuf, time::Instant};

use specs::Component;
use tokio::fs;

use crate::plugins::*;
use specs::storage::DenseVecStorage;

use super::{ThunkContext, CancelToken};

/// This component facilitates bringing file content into the system
/// The listen trait converts completed transfers into file blocks, i.e.
/// ``` filename.ext file
/// add content   .bin
/// add file_src  .text
/// ... (etc)
/// The thunk trait reads files and converts into a binary attribute
#[derive(Component, Default)]
#[storage(DenseVecStorage)]
pub struct OpenFile;

impl Listen for OpenFile {
    fn listen(runtime: &mut crate::Runtime, world: &World) -> Option<AttributeGraph> {
        if let Some(mut thunk_context) = runtime.listen::<Self>(world) {
            if thunk_context.as_ref().contains_attribute("file_src") {
                let block_name = thunk_context.block.block_name.to_string();
                Some(
                    thunk_context
                        .as_mut()
                        .with_text("block_name", block_name)
                        .with_text("block_symbol", "file")
                        .to_owned()
                )
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl Plugin<ThunkContext> for OpenFile {
    fn symbol() -> &'static str {
        "open_file"
    }

    fn description() -> &'static str {
        "Open and reads a file to a string, and then imports to a binary attribute."
    }

    fn call_with_context(context: &mut ThunkContext) -> Option<(tokio::task::JoinHandle<ThunkContext>, CancelToken)> {
        context.clone().task(|_|{
            let mut tc = context.clone();

            async {            
                let start = Instant::now();
                if let Some(file_src) = tc.as_ref().find_text("file_src") {
                    tc.update_status_only("file source found").await;

                    let path_buf = PathBuf::from(&file_src);
                    let file_name = path_buf.file_name().unwrap_or_default().to_str().unwrap_or_default();
                    let file_ext = path_buf.extension()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or_default()
                        .to_string();

                    // block names are usually strictly symbols, but with a # prefix the rules are more relaxed
                    tc.block.block_name = format!("{}", file_name);

                    if !tc.as_ref().contains_attribute("content") || tc.as_ref().is_enabled("refresh").unwrap_or_default(){
                        if let Some(content) = fs::read_to_string(&path_buf).await.ok() {
                            if let Some(project) = tc.project.as_mut() {
                                *project = project.with_block(file_name, "file", |c| {
                                    c.with_text("file_name", file_name)
                                    .with_text("file_ext", file_ext) 
                                    .add_binary_attr("content", content.as_bytes());
                                });
                            }
                        }
                    } else {
                        tc.update_status_only("content found, refresh disabled, skipping read").await;
                        if let Some(content) = tc.as_ref().find_binary("content") {
                            if let Some(project) = tc.project.as_mut() {
                                *project = project.with_block(file_name, "file", |c| {
                                    c.with_text("file_name", file_name)
                                    .with_text("file_ext", file_ext) 
                                    .add_binary_attr("content", content);
                                });
                            }
                        }
                    } 
                }
                tc.as_mut().add_text_attr("elapsed", format!("{:?}", start.elapsed()));
                Some(tc)
            }
        })
    }
}