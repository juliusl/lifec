use std::collections::btree_map::IterMut;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use imgui::Ui;
use specs::Component;
use specs::storage::HashMapStorage;


use crate::state::AttributeGraph;

use super::BlockContext;

#[derive(Default, Component, Clone)]
#[storage(HashMapStorage)]
pub struct Project {
    source: AttributeGraph,
    block_index: BTreeMap<String, BlockContext>
}

impl Project {
    pub fn load_file(path: impl AsRef<str>) -> Option<Project> {
        if let Some(source) = AttributeGraph::load_from_file(path) {
            Some(Self::from(source))
        } else {
            None
        }
    }

    pub fn find_block_mut(&mut self, block_name: impl AsRef<str>) -> Option<&mut BlockContext> {
        self.block_index.get_mut(block_name.as_ref())
    }

    /// iterate through each block_name
    pub fn iter_block_mut(&mut self) -> IterMut<String, BlockContext> {
        self.block_index.iter_mut()
    }

    /// returns a filtered vectored of selected blocks
    pub fn select_blocks(&mut self, select: impl Fn(&String, &BlockContext) -> bool) -> Vec<(String, BlockContext)> {
        self.block_index.iter()
            .filter(|(name, block)| select(name, block))
            .map(|(n, b)| (n.to_string(), b.clone()))
            .collect()
    }

    /// shows the project menu 
    pub fn edit_project_menu(&mut self, ui: &Ui) {
        self.source.edit_attr_menu(ui);

        for (_, block) in self.iter_block_mut() {
            block.edit_menu(ui);
        }

        ui.menu("File", ||{
            if let Some(token) = ui.begin_menu("Export") {
                self.export_blocks_view(ui);
                token.end();
            }

            if let Some(token) = ui.begin_menu("Import") {

                token.end();
            }

            ui.separator();
        });
    }

    /// shows export block view
    pub fn export_blocks_view(&mut self, ui: &Ui) {
        for (block_name, block) in self.iter_block_mut() {
            block.as_mut().edit_attr(format!("Select {}", block_name), "project_selected", ui);
            if ui.is_item_hovered() {
                ui.tooltip(||{
                    ui.text("Preview:");
                    ui.disabled(true, ||{
                        block.as_mut().edit_form_block(ui);
                    });
                });
            }
        }

        let selected = self.select_blocks(|_, context| {
            context.as_ref().is_enabled("project_selected").unwrap_or_default()
        });

        if ui.button("Export selected") {
            if let Some(content) = BlockContext::transpile_blocks(selected).ok() {
                match fs::write(format!("{}-exported.runmd", self.source.hash_code()), content) {
                    Ok(_) => {
                    },
                    Err(_) => {
                    },
                }
            }
        }
    }

    /// imports a new block to the project, returns true if the block was imported
    /// if the block already exists, this method returns false
    pub fn import_block(&mut self, mut block_context: BlockContext) -> bool {
        if !self.block_index.contains_key(&block_context.block_name) {
            let block_name = block_context.block_name.to_string();

            block_context.as_mut().with_bool("project_selected", false);

            self.block_index.insert(block_name, block_context);
            true
        } else {
            false
        }
    }
}

impl From<AttributeGraph> for Project {
    fn from(mut source: AttributeGraph) -> Self {
        let mut project = Project::default();
        let mut block_names = BTreeSet::default();

        // find all block names
        for block in source.iter_blocks() {
            if let Some(block_name) = block.find_text("block_name") {
                block_names.insert(block_name);
            }
        }

        for block_name in block_names.iter() {
            let mut block = BlockContext::root_context(&mut source, block_name);
            block.as_mut().add_bool_attr("project_selected", false);

            project.block_index.insert(block_name.to_string(), block);
        }

        project.source = source;
        project
    }
}

impl AsRef<AttributeGraph> for Project {
    fn as_ref(&self) -> &AttributeGraph {
        &self.source
    }
}

impl AsMut<AttributeGraph> for Project {
    fn as_mut(&mut self) -> &mut AttributeGraph {
        &mut self.source
    }
}