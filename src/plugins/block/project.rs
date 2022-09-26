use super::BlockContext;
use crate::AttributeIndex;
use crate::plugins::ThunkContext;
use crate::state::AttributeGraph;
use atlier::system::Value;
use imgui::Ui;
use specs::storage::HashMapStorage;
use specs::Component;
use tracing::{event, Level};
use std::collections::btree_map::{Iter, IterMut};
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Error;
use std::fmt::Write;
use std::fs;
use std::hash::{Hash, Hasher};

#[derive(Debug, Default, Component, Clone)]
#[storage(HashMapStorage)]
pub struct Project {
    source: AttributeGraph,
    block_index: BTreeMap<String, BlockContext>,
}

impl Project {
    /// reads the .runmd file in the current directory and creates a project
    /// If the file is missing or cannot be parsed this method returns None.
    pub fn runmd() -> Option<Self> {
        Self::load_file(".runmd")
    }

    /// Configures a thunk context from a block in the project
    ///
    /// The block is found by using the block name from the thunk context's block context,
    /// and from the attribute `plugin_symbol` if set. If not set, and a config_block is found,
    /// will use the current graph state returned by config_block.as_ref()
    ///
    pub fn configure(&self, tc: &mut ThunkContext) {
        // if let Some(config_block) = self.find_block(&tc.block.block_name) {
        //     let config = if let Some(event_symbol) = tc.as_ref().find_text("event_symbol") {
        //         event!(Level::TRACE, "Project is configuring from block {}, {event_symbol}", tc.block.block_name);
        //         config_block.get_block(event_symbol).unwrap_or(config_block.as_ref().clone())
        //     } else {
        //         config_block.as_ref().clone()
        //     };

        //     for (name, value) in config.iter_attributes().filter_map(|a| {
        //         if a.is_stable() {
        //             Some((a.name(), a.value()))
        //         } else {
        //             None
        //         }
        //     }) {
        //         tc.as_mut().with(name, value.clone());
        //     }

        //     for a in config
        //         .iter_attributes()
        //         .filter(|a| !a.is_stable())
        //     {
        //         if let Some((_, value)) = a.transient() {
        //             if let Value::Symbol(symbol) = a.value() {
        //                 let symbol = symbol.trim_end_matches("::");
        //                 let name = a.name().trim_end_matches(&format!("::{symbol}"));

        //                 tc.as_mut().define(name, symbol).edit_as(value.clone());
        //             }
        //         }
        //     }

        //     tc.block.block_name = config_block.block_name.to_string();
        // }
        
        todo!()
    }

    pub fn index_hash_code(&self) -> u64 {
        let mut hasher = DefaultHasher::default();
        let hasher = &mut hasher;
        for (name, block) in self.block_index.iter() {
            name.hash(hasher);
            block.hash(hasher);
        }
        hasher.finish()
    }

    pub fn reload_source(&self) -> Self {
        Project::from(self.as_ref().clone())
    }

    pub fn load_file(path: impl AsRef<str>) -> Option<Project> {
        if let Some(source) = AttributeGraph::load_from_file(&path) {
            Some(Self::from(source))
        } else {
            None
        }
    }

    /// Interprets content and returns a project
    pub fn load_content(content: impl AsRef<str>) -> Option<Project> {
        let mut graph = AttributeGraph::from(0);
        if graph.batch_mut(content.as_ref()).is_ok() {
            Some(Project::from(graph))
        } else {
            None
        }
    }

    pub fn find_block_mut(&mut self, block_name: impl AsRef<str>) -> Option<&mut BlockContext> {
        self.block_index.get_mut(block_name.as_ref())
    }

    pub fn find_block(&self, block_name: impl AsRef<str>) -> Option<BlockContext> {
        self.block_index
            .get(block_name.as_ref())
            .and_then(|b| Some(b.to_owned()))
    }

    /// iterate through each block_name
    pub fn iter_block_mut(&mut self) -> IterMut<String, BlockContext> {
        self.block_index.iter_mut()
    }

    /// iterate through each block_name
    pub fn iter_block(&self) -> Iter<String, BlockContext> {
        self.block_index.iter()
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

        // for block_name in block_names.iter() {
        //     let mut block = BlockContext::root_context(&mut source, block_name);
        //     block.as_mut().add_bool_attr("project_selected", false);

        //     project.block_index.insert(block_name.to_string(), block);
        // }

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
