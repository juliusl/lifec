use crate::{plugins::{Plugin, Project}, AttributeGraph};

use super::ThunkContext;

/// The Runmd plugin reads .runmd content into block symbols for the current content
pub struct Runmd;

impl Plugin<ThunkContext> for Runmd {
    fn symbol() -> &'static str {
        "runmd"
    }

    fn description() -> &'static str {
        "Converts a .runmd file into block symbols, then updates the current block."
    }

    fn call_with_context(context: &mut ThunkContext) -> Option<tokio::task::JoinHandle<ThunkContext>> {
        context.clone().task(||{
            let mut tc = context.clone();
            async {
                if let Some(file_block) = tc.block.get_block("file") {
                    if let Some(file_ext) = file_block.find_text("file_ext") {
                        if file_ext == "runmd" {
                            if let Some(content) = file_block.find_binary("content") {
                                  let graph = AttributeGraph::from(content);
                                  let project = Project::from(graph);

                                  if let Some(block) = project.find_block(&tc.block.block_name)  {
                                        for (block_symbol, graph) in block.to_blocks() {
                                            if !tc.block.add_block(&block_symbol, |g|{
                                                *g = graph;
                                            }) {
                                                tc.block.replace_block(&block, block_symbol);
                                            }
                                        }
                                  }
                            }
                        }
                    }
                }

                // TODO can handle some other cases, such as file_path, etc
                Some(tc)
            }
        })
    }
}
 