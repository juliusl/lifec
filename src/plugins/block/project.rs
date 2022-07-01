use super::BlockContext;
use crate::state::AttributeGraph;
use crate::RuntimeDispatcher;
use imgui::Ui;
use specs::storage::HashMapStorage;
use specs::Component;
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
    pub fn index_hash_code(&self) -> u64 {
        let mut hasher = DefaultHasher::default();
        let hasher = &mut hasher;
        for (name, block) in self.block_index.iter() {
            name.hash(hasher);
            block.hash(hasher);
        }
        hasher.finish()
    }

    pub fn transpile(&self) -> Result<String, Error> {
        let mut src = String::default();

        match self.transpile_root() {
            Ok(root) => {
                writeln!(src, "{}", root)?;
            }
            Err(_) => {}
        }

        for (_, block) in self.block_index.iter() {
            match block.transpile() {
                Ok(block) => {
                    writeln!(src, "{}", block)?;
                }
                Err(_) => todo!(),
            }
        }

        Ok(src)
    }

    pub fn transpile_root(&self) -> Result<String, Error> {
        let mut src = String::new();
        writeln!(src, "```")?;
        for attr in self.as_ref().iter_attributes().filter(|a| a.id() == 0) {
            if attr.name().starts_with("block_") {
                continue;
            }

            if attr.is_stable() {
                BlockContext::transpile_value(&mut src, "add", attr.name(), attr.value())?;
            } else {
                let symbols: Vec<&str> = attr.name().split("::").collect();
                let a = symbols.get(0);
                let b = symbols.get(1);

                if let (Some(a), Some(b)) = (a, b) {
                    writeln!(src, "define {} {}", a, b)?;
                    if let Some((name, value)) = attr.transient() {
                        BlockContext::transpile_value(&mut src, "edit", name, value)?;
                    }
                }
            }
        }
        writeln!(src, "```")?;
        Ok(src)
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

    pub fn replace_block(&mut self, mut block_context: BlockContext) -> bool {
        let block_name = block_context.block_name.to_string();
        block_context.as_mut().with_bool("project_selected", false);

        if let Some(removed) = self.block_index.insert(block_name.clone(), block_context) {
            if let Some(next) = self.find_block_mut(block_name) {
                if let Some(enabled) = removed.as_ref().is_enabled("project_selected") {
                    next.as_mut().with_bool("project_selected", enabled);
                }
            }
            true
        } else {
            false
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
                        block.edit_block_tooltip_view(false, ui);
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

    /// imports the graph in it's current context as a block with the block_name/block_symbol values currently set
    pub fn import(&mut self, graph: AttributeGraph) -> bool {
        if let (Some(block_name), Some(block_symbol)) = (
            graph.find_text("block_name"),
            graph.find_text("block_symbol"),
        ) {
            let mut importing = AttributeGraph::from(0);
            importing.start_block_mode(&block_name, block_symbol);
            importing.copy(&graph);
            self.import_block(BlockContext::from(importing))
        } else {
            eprintln!("Did not find text attributes for block_name and block_symbol");
            false
        }
    }

    /// Returns a new project, defining a new block
    pub fn with_block(
        &self,
        block_name: impl AsRef<str>,
        block_symbol: impl AsRef<str>,
        config: impl FnOnce(&mut AttributeGraph),
    ) -> Self {
        let mut next = self.clone();
        next.as_mut().start_block_mode(block_name, block_symbol);
        config(next.as_mut());
        next.as_mut().end_block_mode();
        next.reload_source()
    }

    /// sends a message between two blocks within the project
    /// returns the transpilation of the project without applying the events
    pub fn send(
        &self,
        from: impl AsRef<str>,
        to: impl AsRef<str>,
        event_name: impl AsRef<str>,
        message: impl AsRef<str>,
    ) -> Option<String> {
        println!(
            "Sending event {} {}->{}",
            event_name.as_ref(),
            from.as_ref(),
            to.as_ref()
        );
        let from = self.find_block(from.as_ref());
        let to = self.find_block(to.as_ref());

        if let (Some(from), Some(to)) = (from, to) {
            let mut update = AttributeGraph::from(0);
            if let Some(from) = from.transpile().ok() {
                match update.batch_mut(from) {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
            if let Some(to) = to.transpile().ok() {
                match update.batch_mut(to) {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }

            let mut update = Project::from(update);
            update.as_mut().add_event(event_name, message);
            update
                .as_mut()
                .with_text("from", from.block_name)
                .with_text("to", to.block_name);
            update.transpile().ok()
        } else {
            None
        }
    }

    pub fn receive(
        &self,
        update: impl AsRef<str>,
        dest_block: impl AsRef<str>,
    ) -> Option<BlockContext> {
        if let Some(mut graph) = AttributeGraph::from(0).batch(update.as_ref()).ok() {
            println!("Received update -> {}", dest_block.as_ref());
            graph.apply_events();
            let update = Project::from(graph);
            update.find_block(dest_block)
        } else {
            None
        }
    }

    pub fn receive_mut(
        &mut self,
        update: impl AsRef<str>,
        dest_block: impl AsRef<str>,
        received: impl FnOnce(&mut BlockContext),
    ) -> bool {
        match self.as_mut().batch_mut(update) {
            Ok(_) => {
                self.as_mut().apply_events();
                *self = self.reload_source();
                if let Some(dest) = self.find_block_mut(&dest_block) {
                    println!("Received update -> {}", dest_block.as_ref());
                    received(dest);
                    true
                } else {
                    false
                }
            }
            Err(_) => false,
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
    ```
    "#;

    match AttributeGraph::from(0).batch(test) {
        Ok(graph) => {
            let mut graph = Project::from(graph);
            if let Some(message) = graph.send(
                "sh_test",
                "println_settings",
                "connect",
                r#"
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
            "#,
            ) {
                let update = Project::from(AttributeGraph::from(0).batch(&message).expect("works"));

                let from_block = update
                    .as_ref()
                    .find_text("from")
                    .and_then(|f| update.find_block(f));
                let to_block = update
                    .as_ref()
                    .find_text("to")
                    .and_then(|f| update.find_block(f));

                assert!(from_block.is_some());
                assert!(to_block.is_some());

                println!("from: {}", from_block.expect("exists").block_name);
                println!("to: {}", to_block.expect("exists").block_name);

                assert!(graph.receive_mut(message, "println_settings", |received| {
                    println!("{}", received.transpile().expect("should exist"));
                }));
            } else {
                assert!(false, "should exist");
            }
        }
        Err(err) => assert!(false, "should be able to dispatch test, {:?}", err),
    }
}

#[test]
fn test_with_block() {
    let project = Project::default();

    let project = project.with_block("test.sh", "file", |a| {
        a.with_binary("content", b"test message".to_vec());
    });

    assert!(project.find_block("test.sh").is_some());
    assert!(project
        .find_block("test.sh")
        .and_then(|b| b.get_block("file"))
        .is_some());

    let project = project.with_block("test.sh", "file", |a| {
        a.with_text("file_src", "/some/test/path");
    });

    assert!(project.find_block("test.sh").is_some());
    assert!(project
        .find_block("test.sh")
        .and_then(|b| b.get_block("file"))
        .is_some());

    if let Some(file_block) = project
        .find_block("test.sh")
        .and_then(|b| b.get_block("file"))
    {
        assert!(file_block.find_text("file_src").is_some());
        assert!(file_block.find_binary("content").is_some());
    }

    let project = project.with_block("test.sh", "interpret", |a| {
        a.with_text("result", "interpretation");
    });

    assert!(project.find_block("test.sh").is_some());
    assert!(project
        .find_block("test.sh")
        .and_then(|b| b.get_block("interpret"))
        .is_some());
    assert!(project
        .find_block("test.sh")
        .and_then(|b| b.get_block("file"))
        .is_some());

    if let Some(file_block) = project
        .find_block("test.sh")
        .and_then(|b| b.get_block("file"))
    {
        assert!(file_block.find_text("file_src").is_some());
        assert!(file_block.find_binary("content").is_some());
    }

    if let Some(file_block) = project
        .find_block("test.sh")
        .and_then(|b| b.get_block("interpret"))
    {
        assert!(file_block.find_text("result").is_some());
    }

    println!("{:#?}", project);
}
