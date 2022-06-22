use super::BlockContext;
use crate::state::AttributeGraph;
use atlier::system::Value;
use imgui::Ui;
use specs::storage::HashMapStorage;
use specs::Component;
use std::collections::btree_map::IterMut;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::str::from_utf8;

#[derive(Debug, Default, Component, Clone)]
#[storage(HashMapStorage)]
pub struct Project {
    source: AttributeGraph,
    block_index: BTreeMap<String, BlockContext>,
}

impl Project {
    pub fn reload_source(&self) -> Self {
        Project::from(self.as_ref().clone())
    }

    pub fn load_file(path: impl AsRef<str>) -> Option<Project> {
        if let Some(source) = AttributeGraph::load_from_file(&path) {
            let path = format!("{}.journal", path.as_ref());
            source.write_file(path, "journal");

            Some(Self::from(source))
        } else {
            None
        }
    }

    pub fn replace_block(&mut self, mut block_context: BlockContext) -> bool {
        let block_name = block_context.block_name.to_string();

        block_context.as_mut().with_bool("project_selected", false);

        self.block_index.insert(block_name, block_context).is_some()
    }

    pub fn find_block_mut(&mut self, block_name: impl AsRef<str>) -> Option<&mut BlockContext> {
        self.block_index.get_mut(block_name.as_ref())
    }

    /// iterate through each block_name
    pub fn iter_block_mut(&mut self) -> IterMut<String, BlockContext> {
        self.block_index.iter_mut()
    }

    /// returns a filtered vectored of selected blocks
    pub fn select_blocks(
        &mut self,
        select: impl Fn(&String, &BlockContext) -> bool,
    ) -> Vec<(String, BlockContext)> {
        self.block_index
            .iter()
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

        ui.menu("File", || {
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
            block
                .as_mut()
                .edit_attr(format!("Select {}", block_name), "project_selected", ui);
            if ui.is_item_hovered() {
                ui.tooltip(|| {
                    ui.text("Preview:");
                    ui.disabled(true, || {
                        block.as_mut().edit_form_block(ui);

                        if let Some(Value::BinaryVector(journal)) =
                            block.as_ref().find_attr_value("journal")
                        {
                            if let Some(journal) = from_utf8(journal).ok() {
                                ui.text_wrapped(journal);
                            }
                        }
                    });
                });
            }
        }

        let selected = self.select_blocks(|_, context| {
            context
                .as_ref()
                .is_enabled("project_selected")
                .unwrap_or_default()
        });

        if ui.button("Export selected") {
            if let Some(content) = BlockContext::transpile_blocks(selected).ok() {
                match fs::write(
                    format!("{}-exported.runmd", self.source.hash_code()),
                    content,
                ) {
                    Ok(_) => {}
                    Err(_) => {}
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

#[test]
fn test_send_event() {
    use crate::RuntimeDispatcher;

    let test = r#"
    ``` sh_test form
    add command .TEXT sh ./test.sh
    add debug_out .BOOL true
    ```

    ``` sh_test node
    add debug .BOOL true
    add input_label .TEXT accept
    add node_title .TEXT sh_test
    add output_label .TEXT publish
    ```

    ``` sh_test publish
    add called .BOOL true
    add code .INT 0
    add command .TEXT sh ./test.sh
    add elapsed .TEXT 2 ms
    add stderr .BINARY_VECTOR 
    add stdout .BINARY_VECTOR SGVsbG8gV29ybGQK
    add timestamp_local .TEXT 2022-06-20 19:50:07.782710 -07:00
    add timestamp_utc .TEXT 2022-06-21 02:50:07.782701 UTC
    ```

    ``` sh_test thunk
    add thunk_symbol .TEXT process
    ``` 

    ``` println_settings accept
    from sh_test publish called
    from sh_test publish code 
    from sh_test publish command
    from sh_test publish elapsed
    from sh_test publish stderr
    from sh_test publish stdout
    from sh_test publish timestamp_local
    from sh_test publish timestamp_utc
    ```
    "#;

    match AttributeGraph::from(0).batch(test) {
        Ok(graph) => {
            let mut graph = Project::from(graph);

            let message = r#"
        ``` println_settings accept
        find_remove called
        find_remove code
        find_remove stdout
        find_remove stderr
        find_remove elapsed
        find_remove timestamp_utc
        find_remove timestamp_local
        ```
        "#;

            graph.as_mut().add_event("disconnect", message);
            graph.as_mut().apply_events();

            if let Some(accept) = graph.find_block_mut("println_settings") {
                println!("{:#?}", accept.as_ref());
            }
        }
        Err(_) => assert!(false, "should be able to dispatch test"),
    }
}
