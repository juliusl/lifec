mod transpile;
use transpile::Transpile;

mod project;
pub use project::Project;

use imgui::{Ui, MenuItem};
use std::{collections::BTreeSet, fmt::Error};
use std::fmt::Write;
use atlier::system::Value;
use specs::Component;
use specs::storage::DenseVecStorage;
use crate::{AttributeGraph};

use super::Plugin;

/// BlockContext provides common methods for working with blocks
#[derive(Component, Default, Clone)]
#[storage(DenseVecStorage)]
pub struct BlockContext {
    graph: AttributeGraph,
    block_name: String,
    block_symbols: BTreeSet<String>,
    max_block_id: u32
}

impl BlockContext {
    pub fn transpile_blocks(blocks: Vec<(String, BlockContext)>) -> Result<String, Error> {
        let mut output = String::new(); 

        for (_, context) in blocks { 
            match context.transpile() {
                Ok(transpiled) => {
                    writeln!(output, "{}", transpiled)?;
                },
                Err(err) => {
                    return Err(err);
                },
            }
        }

        Ok(output)
    }

    /// merge the block symbol of another block context, returns true if a change happend
    pub fn merge_block(&mut self, other: &BlockContext, block_symbol: impl AsRef<str>) -> bool {
        if let Some(update) = other.get_block(block_symbol.as_ref()) {
            let current = self.as_ref().hash_code();

            if self.update_block(block_symbol, |updating| {
                if updating.hash_code() != update.hash_code() {
                    updating.merge(&update);
                }
            }) {
                current != self.as_ref().hash_code()
            } else {
                false 
            }
        } else {
            false
        }
    }

    /// returns the block name of the current context
    pub fn block_name(&self) -> Option<String> {
        self.as_ref().find_text("block_name")
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
    pub fn update_block(&mut self, block_symbol: impl AsRef<str>, update: impl FnOnce(&mut AttributeGraph)) -> bool {
        if let Some(mut block) = self.get_block(block_symbol) {
            update(&mut block);
            self.as_mut().merge(&block);
            true 
        } else {
            false
        }
    }

    /// adds a new block, returns true if a new block was added, does not call configure if the block exists
    pub fn add_block(&mut self, block_symbol: impl AsRef<str>, configure: impl FnOnce(&mut AttributeGraph)) -> bool {
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
                        write!(src, "add {} ", attr.name())?;
                        Self::transpile_value(&mut src, attr.value())?;
                    } else {
                        if let Some((name, value)) = attr.transient() {
                            if name != &format!("{}::{}", self.block_name, symbol) {
                                write!(src, "edit {} {} ", attr.name(), name)?;
                                Self::transpile_value(&mut src, value)?;
                            }
                        }
                    }
                }
            }
            writeln!(src, "```")?;
            writeln!(src, "")?;
        }
        Ok(src)
    }

    pub fn transpile_value(src: &mut String, value: &Value) -> Result<(), Error>{
        match value {
            atlier::system::Value::Empty => {
                writeln!(src, ".EMPTY")?;
            },
            atlier::system::Value::Bool(val) => {
                writeln!(src, ".BOOL {}", val)?;
            },
            atlier::system::Value::TextBuffer(text) => {
                writeln!(src, ".TEXT {}", text)?;
            },
            atlier::system::Value::Int(val) => {
                writeln!(src, ".INT {}", val)?;
            },
            atlier::system::Value::IntPair(val1, val2) => {
                writeln!(src, ".INT_PAIR {}, {}", val1, val2)?;
            },
            atlier::system::Value::IntRange(val1, val2, val3) => {
                writeln!(src, ".INT_RANGE {}, {}, {}", val1, val2, val3)?;
            },
            atlier::system::Value::Float(val) => {
                writeln!(src, ".FLOAT {}", val)?;
            },
            atlier::system::Value::FloatPair(val1, val2) => {
                writeln!(src, ".FLOAT_PAIR {}, {}", val1, val2)?;
            },
            atlier::system::Value::FloatRange(val1, val2, val3) => {
                writeln!(src, ".FLOAT_RANGE {}, {}, {}", val1, val2, val3)?;
            },
            atlier::system::Value::BinaryVector(bin) => {
                writeln!(src, ".BINARY_VECTOR {}", base64::encode(bin))?;
            },
            atlier::system::Value::Reference(val) => {
                writeln!(src, ".REFERENCE {}", val)?;
            },
            atlier::system::Value::Symbol(val) => {
                writeln!(src, ".SYMBOL {}", val)?;
            },
        }

        Ok(())
    }

    pub fn edit_menu(&mut self, ui: &Ui) {
        let block_name = self.block_name.clone();
        if let Some(token) = ui.begin_menu("File") {
            if let Some(token) = ui.begin_menu("Blocks") {
                if MenuItem::new(format!("Transpile {0} to {0}.runmd", block_name)).build(ui) {
                    self.add_block(
                        "file", 
                        |f| 
                        f.add_text_attr("runmd_path", format!("{}.runmd", block_name)
                    ));
                    Transpile::call_with_context(self);
                }
                token.end();
            }
            token.end();
        }
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
```
    "#;
    assert!(test_graph.batch_mut(test).is_ok());

    let mut sh_test = BlockContext::root_context(&test_graph, "sh_test");
    
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
        },
        Err(_) => todo!(),
    }
}