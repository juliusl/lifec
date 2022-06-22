mod transpile;
use serde::{Deserialize, Serialize};
use transpile::Transpile;

mod project;
pub use project::Project;

use crate::{AttributeGraph, RuntimeDispatcher};
use atlier::system::{Attribute, Value};
use imgui::{ChildWindow, MenuItem, Ui};
use specs::storage::DenseVecStorage;
use specs::Component;
use std::fmt::Write;
use std::str::from_utf8;
use std::{collections::BTreeSet, fmt::Error};

use super::Plugin;

/// BlockContext provides common methods for working with blocks
#[derive(Debug, Component, Default, Clone, Hash, PartialEq, Serialize, Deserialize)]
#[storage(DenseVecStorage)]
pub struct BlockContext {
    graph: AttributeGraph,
    pub block_name: String,
    block_symbols: BTreeSet<String>,
    max_block_id: u32,
}

impl BlockContext {
    /// creates an index entry for block_symbol
    pub fn write_index(
        &mut self,
        block_symbol: impl AsRef<str>,
        symbol: impl AsRef<str>,
        index_name: impl AsRef<str>,
        index_value: Value,
    ) -> bool {
        let block_name = self.block_name.to_string();
        let to_index = (index_name.as_ref().to_string(), index_value.clone());

        self.update_block(block_symbol.as_ref(), |updating| {
            let index_entry = updating.define(&block_name, &block_symbol);
            let mut index_entry = index_entry.to_owned();
            index_entry.edit_as(Value::Symbol(format!(
                "{}::{}",
                block_symbol.as_ref(),
                symbol.as_ref()
            )));
            index_entry.commit();
            index_entry.edit(to_index.clone());
            updating.add_attribute(index_entry);
        })
    }

    /// reads the index directly for a string/value pair by block_symbol/symbol pair
    pub fn read_index(
        &self,
        block_symbol: impl AsRef<str>,
        symbol: impl AsRef<str>,
    ) -> Option<(String, Value)> {
        if let Some(block) = self.get_block(&block_symbol) {
            block.read_block_index(
                block.entity(),
                &self.block_name,
                block_symbol.as_ref(),
                symbol,
            )
        } else {
            None
        }
    }

    pub fn transpile_blocks(blocks: Vec<(String, BlockContext)>) -> Result<String, Error> {
        let mut output = String::new();

        for (_, context) in blocks {
            match context.transpile() {
                Ok(transpiled) => {
                    writeln!(output, "{}", transpiled)?;
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
        Ok(output)
    }

    /// replace a block by symbol, using the block of another block context
    pub fn replace_block(&mut self, other: &BlockContext, block_symbol: impl AsRef<str>) -> bool {
        if let Some(update) = other.get_block(block_symbol.as_ref()) {
            self.update_block(&block_symbol, |graph| {
                *graph = update;
            })
        } else {
            false
        }
    }

    /// returns the block name of the current context
    pub fn block_name(&self) -> Option<String> {
        self.as_ref().find_text("block_name")
    }

    pub fn has_pending_events(&self) -> bool {
        let mut has_event = !self.graph.find_symbols("event").is_empty();

        for symbol in self.block_symbols.iter() {
            if let Some(graph) = self.get_block(symbol) {
                has_event |= !graph.find_symbols("event").is_empty();
            }
        }

        has_event
    }

    /// create a root context for a given block name from a source graph
    pub fn root_context(source: &AttributeGraph, block_name: impl AsRef<str>) -> Self {
        // each block represents a component of block_name
        let mut root = AttributeGraph::from(0);
        let mut symbols = BTreeSet::default();
        let mut max_block_id = 0;
        root.with_text("block_name", block_name.as_ref());
        for block in source.find_blocks_for(&block_name) {
            if let Some(block_symbol) = block.find_text("block_symbol") {
                source.include_block(&mut root, &block_symbol);
                symbols.insert(block_symbol.to_string());
            }

            if block.entity() > max_block_id {
                max_block_id = block.entity();
            }
        }

        BlockContext {
            graph: root,
            block_symbols: symbols,
            block_name: block_name.as_ref().to_string(),
            max_block_id,
        }
    }

    /// returns a block if it exists within the context
    pub fn get_block(&self, block_symbol: impl AsRef<str>) -> Option<AttributeGraph> {
        if self.block_symbols.contains(block_symbol.as_ref()) {
            self.graph.find_block("", block_symbol)
        } else {
            None
        }
    }

    /// update an existing block, otherwise no-op, returns true if udpate was called
    pub fn update_block(
        &mut self,
        block_symbol: impl AsRef<str>,
        update: impl FnOnce(&mut AttributeGraph),
    ) -> bool {
        if let Some(mut block) = self.get_block(block_symbol) {
            for attr in block.iter_attributes() {
                self.as_mut().remove(attr); 
            }
            update(&mut block);
            self.as_mut().merge(&block);
            true
        } else {
            false
        }
    }

    /// adds a new block, returns true if a new block was added, does not call configure if the block exists
    pub fn add_block(
        &mut self,
        block_symbol: impl AsRef<str>,
        configure: impl FnOnce(&mut AttributeGraph),
    ) -> bool {
        if self.block_symbols.contains(block_symbol.as_ref()) {
            false
        } else {
            self.max_block_id += 1;
            let next = self.max_block_id;

            let mut next_block = AttributeGraph::from(next);
            next_block
                .with_text("block_name", self.block_name.to_string())
                .with_text("block_symbol", block_symbol.as_ref().to_string());

            let block = next_block.define(&self.block_name, &block_symbol.as_ref());
            block.edit_as(Value::Symbol(format!(
                "{}::{}",
                block_symbol.as_ref(),
                "block"
            )));
            block.commit();
            block.edit_as(Value::Int(next as i32));

            self.block_symbols.insert(block_symbol.as_ref().to_string());
            configure(&mut next_block);

            let next_block = &next_block.to_owned();
            self.graph.merge(next_block);
            true
        }
    }

    pub fn transpile(&self) -> Result<String, Error> {
        let mut src = String::new();

        for symbol in self.block_symbols.iter() {
            writeln!(src, "``` {} {}", self.block_name, symbol)?;
            if let Some(block) = self.get_block(symbol) {
                for attr in block.iter_attributes() {
                    if attr.name().starts_with("block_") {
                        continue;
                    }

                    if attr.is_stable() {
                        Self::transpile_value(&mut src, "add", attr.name(), attr.value())?;
                    }
                }
            }
            writeln!(src, "```")?;
            writeln!(src, "")?;
        }
        Ok(src)
    }

    pub fn transpile_value(
        src: &mut String,
        event: impl AsRef<str>,
        name: impl AsRef<str>,
        value: &Value,
    ) -> Result<(), Error> {
        match value {
            atlier::system::Value::Empty => {
                writeln!(src, "{} {} .EMPTY", event.as_ref(), name.as_ref())?;
            }
            atlier::system::Value::Bool(val) => {
                writeln!(src, "{} {} .BOOL {}", event.as_ref(), name.as_ref(), val)?;
            }
            atlier::system::Value::TextBuffer(text) => {
                writeln!(src, "{} {} .TEXT {}", event.as_ref(), name.as_ref(), text)?;
            }
            atlier::system::Value::Int(val) => {
                writeln!(src, "{} {} .INT {}", event.as_ref(), name.as_ref(), val)?;
            }
            atlier::system::Value::IntPair(val1, val2) => {
                writeln!(
                    src,
                    "{} {} .INT_PAIR {}, {}",
                    event.as_ref(),
                    name.as_ref(),
                    val1,
                    val2
                )?;
            }
            atlier::system::Value::IntRange(val1, val2, val3) => {
                writeln!(
                    src,
                    "{} {} .INT_RANGE {}, {}, {}",
                    event.as_ref(),
                    name.as_ref(),
                    val1,
                    val2,
                    val3
                )?;
            }
            atlier::system::Value::Float(val) => {
                writeln!(src, "{} {} .FLOAT {}", event.as_ref(), name.as_ref(), val)?;
            }
            atlier::system::Value::FloatPair(val1, val2) => {
                writeln!(
                    src,
                    "{} {}, .FLOAT_PAIR {}, {}",
                    event.as_ref(),
                    name.as_ref(),
                    val1,
                    val2
                )?;
            }
            atlier::system::Value::FloatRange(val1, val2, val3) => {
                writeln!(
                    src,
                    "{} {} .FLOAT_RANGE {}, {}, {}",
                    event.as_ref(),
                    name.as_ref(),
                    val1,
                    val2,
                    val3
                )?;
            }
            atlier::system::Value::BinaryVector(bin) => {
                writeln!(
                    src,
                    "{} {} .BINARY_VECTOR {}",
                    event.as_ref(),
                    name.as_ref(),
                    base64::encode(bin)
                )?;
            }
            atlier::system::Value::Reference(val) => {
                writeln!(
                    src,
                    "{} {} .REFERENCE {}",
                    event.as_ref(),
                    name.as_ref(),
                    val
                )?;
            }
            atlier::system::Value::Symbol(val) => {
                if !val.ends_with("::block") {
                    write!(src, "define")?;
                    for part in val.split("::") {
                        write!(src, " {}", part)?;
                    }
                    writeln!(src, "")?;
                }
            }
        }

        Ok(())
    }

    pub fn edit_menu(&mut self, ui: &Ui) {
        let block_name = self.block_name.clone();
        if let Some(token) = ui.begin_menu("File") {
            if let Some(token) = ui.begin_menu("Blocks") {
                if MenuItem::new(format!("Transpile {0} to {0}.runmd", block_name)).build(ui) {
                    let mut transpiled = self.clone();
                    transpiled.add_block("file", |f| {
                        f.add_text_attr("runmd_path", format!("{}.runmd", block_name))
                    });
                    Transpile::call_with_context(&mut transpiled);
                }

                if ui.is_item_hovered() {
                    ui.tooltip(|| {
                        self.edit_block_tooltip_view(true, ui);
                    });
                }

                if MenuItem::new(format!("Dump {}", self.block_name)).build(ui) {
                    println!("{:#?}", self.as_ref());
                }

                if self.has_pending_events() {
                    if MenuItem::new(format!("Apply events for {}", self.block_name)).build(ui) {
                        self.resolve_events();
                    }
                }
                token.end();
            }
            token.end();
        }
    }

    /// resolve by applying and completing events
    /// resolving at this scope, is valid for entity scoped events
    pub fn resolve_events(&mut self) {
        for block_symbol in self.block_symbols.clone().iter() {
            if let Some(mut block) = self.get_block(block_symbol) {
                for (name, value) in block.take_symbol_values("event") {
                    println!("Event {}", name);
                    if let Value::BinaryVector(content) = value {
                        if let Some(content) = from_utf8(&content).ok() {
                            println!("Applying\n{}", content);

                            match self.as_mut().batch_mut(content) {
                                Ok(_) => {
                                    println!("Applied event {}", name);
                                    if self.update_block(block_symbol, |block| {
                                        block.find_remove(&name);
                                    }) {
                                        println!("Completing event {}", name);
                                    }
                                },
                                Err(_) => {
                                    eprintln!("could not apply events");
                                },
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn edit_block_view(&mut self, show_transpile_preview: bool, ui: &Ui) {
        ChildWindow::new(&format!("edit_block_view_{}", self.block_name))
            .size([600.0, 420.0])
            .build(ui, || {
                ui.group(|| {
                    for block_symbol in self.block_symbols.clone().iter() {
                        ui.text(format!("{}:", block_symbol));
                        ui.separator();
                        self.edit_block(block_symbol, ui);
                        ui.new_line();
                    }
                });
            });

        if let Some(mut transpiled) = self.transpile().ok() {
            ui.same_line();
            ChildWindow::new(&format!("transpile_preview_{}", self.block_name))
                .size([0.0, 420.0])
                .build(ui, || {
                    if transpiled.is_empty() || !show_transpile_preview {
                        return;
                    }

                    let size = ui.calc_text_size(&transpiled);
                    ui.group(|| {
                        ui.disabled(true, || {
                            ui.input_text_multiline(
                                format!("Preview {}.runmd", self.block_name),
                                &mut transpiled,
                                size,
                            )
                            .build();
                        })
                    });
                });
        }
    }

    pub fn edit_block_tooltip_view(&mut self, show_transpile_preview: bool, ui: &Ui) {
        ui.group(|| {
            for block_symbol in self.block_symbols.clone().iter() {
                ui.text(format!("{}:", block_symbol));
                self.edit_block(block_symbol, ui);
                ui.new_line();
            }
        });

        if let Some(mut transpiled) = self.transpile().ok() {
            ui.same_line();
            if transpiled.is_empty() || !show_transpile_preview {
                return;
            }

            let size = ui.calc_text_size(&transpiled);
            ui.group(|| {
                ui.disabled(true, || {
                    ui.input_text_multiline(
                        format!("Preview {}.runmd", self.block_name),
                        &mut transpiled,
                        size,
                    )
                    .build();
                })
            });
        }
    }

    pub fn edit_block_table_view(&mut self, ui: &Ui) {
        ui.group(|| {
            if let Some(token) = ui.tab_bar(format!("{}_table_view_tab_bar", self.block_name)) {
                for block_symbol in self.block_symbols.clone().iter() {
                    if let Some(token) =
                        ui.tab_item(format!("{} {}", self.block_name, block_symbol))
                    {
                        self.edit_block_table(block_symbol, ui);
                        token.end();
                    }
                }
                token.end();
            }
        });
    }

    pub fn edit_block(&mut self, symbol_name: impl AsRef<str>, ui: &Ui) {
        self.update_block(&symbol_name, |block| {
            let attrs: Vec<&mut Attribute> = Self::iter_block_attrs_mut(block).collect();
            if !attrs.is_empty() {
                for attr in attrs {
                    attr.edit_value("", ui);
                }
            } else {
                ui.text("Empty");
            }
        });
    }

    pub fn edit_block_table(&mut self, symbol_name: impl AsRef<str>, ui: &Ui) {
        self.update_block(&symbol_name, |block| {
            block.edit_attr_table(ui);
        });
    }

    /// creates an iter on all the block attributes skipping block specific attrs
    fn iter_block_attrs_mut(block: &mut AttributeGraph) -> impl Iterator<Item = &mut Attribute> {
        block
            .iter_mut_attributes()
            .filter(|a| {
                !a.name().starts_with("block_")
                    && match a.value() {
                        Value::Symbol(symbol) => !symbol.ends_with("::block"),
                        _ => true,
                    }
            })
            .into_iter()
    }
}

impl From<AttributeGraph> for BlockContext {
    fn from(g: AttributeGraph) -> Self {
        if let Some(block_name) = g.find_text("block_name") {
            Self::root_context(&g, block_name)
        } else {
            Self {
                graph: g,
                ..Default::default()
            }
        }
    }
}

impl AsRef<AttributeGraph> for BlockContext {
    fn as_ref(&self) -> &AttributeGraph {
        &self.graph
    }
}

impl AsMut<AttributeGraph> for BlockContext {
    fn as_mut(&mut self) -> &mut AttributeGraph {
        &mut self.graph
    }
}

#[test]
fn test_block_context() {
    use crate::plugins::NodeContext;
    use crate::RuntimeDispatcher;
    let mut test_graph = AttributeGraph::from(0);

    test_graph.with_text("journal", "".to_string());

    let test = r#"
``` cargo_help node
add node_title .TEXT cargo_help
add debug .BOOL true
``` form
add command .TEXT cargo help
add debug_out .BOOL true
``` thunk
```

``` sh_test node
add node_title .TEXT sh_test
add debug .BOOL true
``` form
add command .TEXT sh ./test.sh
add debug_out .BOOL true
``` thunk
``` event
```
    "#;
    assert!(test_graph.batch_mut(test).is_ok());

    println!("{:#?}", test_graph);

    let mut sh_test = BlockContext::root_context(&test_graph, "sh_test");

    assert!(sh_test.write_index("event", "from", "test::publish", Value::Empty));
    println!("{:#?}", sh_test.get_block("event"));

    if let Some((name, Value::Empty)) = sh_test.read_index("event", "from") {
        assert_eq!(name, "test::publish".to_string());
    } else {
        assert!(false, "should exist")
    }

    let sh_test_command = sh_test
        .get_block("form")
        .and_then(|a| a.find_text("command"));

    assert_eq!(sh_test_command, Some("sh ./test.sh".to_string()));

    sh_test.update_block("thunk", |g| {
        g.with_bool("enabled", false);
    });

    assert!(sh_test.add_block("accept", |attr| {
        attr.add_empty_attr("filename");
    }));

    let other_context = NodeContext::from(sh_test.as_ref().clone());
    let back_to_block = BlockContext::from(other_context.as_ref().clone());

    println!("{:#?}", back_to_block.as_ref());
    match back_to_block.transpile() {
        Ok(result) => {
            println!("{}", result);
        }
        Err(_) => todo!(),
    }
}
