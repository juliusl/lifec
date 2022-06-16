use atlier::system::Value;
use specs::Component;
use specs::storage::HashMapStorage;
use crate::AttributeGraph;

/// file context represents one file
#[derive(Component, Clone, Default)]
#[storage(HashMapStorage)]
pub struct FileContext(AttributeGraph);

impl FileContext {
    pub fn file_name(&self) -> Option<String>  {
        self.as_ref().find_text("block_name")
    }

    pub fn has_content(&self) -> bool {
        if let Some(Value::BinaryVector(content)) = self.as_ref().find_attr_value("content") {
            !content.is_empty()
        } else {
            false 
        }
    }
}

impl AsRef<AttributeGraph> for FileContext {
    fn as_ref(&self) -> &AttributeGraph {
        &self.0
    }
}

impl AsMut<AttributeGraph> for FileContext {
    fn as_mut(&mut self) -> &mut AttributeGraph {
        &mut self.0 
    }
}

impl From<AttributeGraph> for FileContext {
    fn from(graph: AttributeGraph) -> Self {
        Self(graph)
    }
}