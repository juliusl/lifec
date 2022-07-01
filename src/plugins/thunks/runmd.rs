use specs::Component;
use specs::storage::HashMapStorage;

use crate::{plugins::{Plugin, Project}, AttributeGraph};

use super::ThunkContext;

/// The Runmd plugin reads .runmd content into block symbols for the current content
#[derive(Component, Default)]
#[storage(HashMapStorage)]
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
                let project = Project::from(tc.as_ref().clone());

                for (block_name, block) in project.iter_block() {
                    let content = block.get_block("file")
                        .filter(|b| b.find_text("file_ext").filter(|ext| ext != "runmd").is_some())
                        .and_then(|file_block| {
                            file_block.find_binary("content")
                        });

                    if let Some(content) = content {
                        let graph = AttributeGraph::from(content);
                        let project = Project::from(graph);
                        tc.as_mut()
                            .add_message(
                                block_name, 
                                "project", 
                                project.transpile().unwrap_or_default());
                    }
                }

                Some(tc)
            }
        })
    }
}
 