use crate::prelude::{Event, Plugins, Runtime, ThunkContext};
use atlier::system::Value;
use reality::{Block, SpecialAttribute};
use specs::prelude::*;
use specs::{Entities, ReadStorage, SystemData, WorldExt};

/// Special attribute to define an operation in the root block for the workspace,
///
#[derive(SystemData)]
pub struct Operations<'a>(
    Plugins<'a>,
    Entities<'a>,
    ReadStorage<'a, Block>,
    ReadStorage<'a, Event>,
);

impl<'a> Operations<'a> {
    /// Returns a map of operations and their
    ///
    pub fn scan_root(&self) -> Vec<(String, crate::prelude::Event)> {
        let Operations(.., entities, blocks, events) = self;

        let mut operations = vec![];

        let root_block = entities.entity(0);

        if let Some(block) = blocks.get(root_block) {
            for operation in block
                .index()
                .iter()
                .filter(|b| b.root().name().ends_with("operation"))
            {
                let operation_entity = operation.root().id();
                let operation_entity = entities.entity(operation_entity);

                let mut event = events
                    .get(operation_entity)
                    .expect("should have an event")
                    .clone();

                event.set_name(
                    operation
                        .find_property("name")
                        .expect("should have a name")
                        .symbol()
                        .expect("should be a symbol"),
                );

                operations.push((operation.root().name().to_string(), event));
            }
        }

        operations
    }

    /// Executes an operation from the root block,
    ///
    pub fn execute_operation(
        &mut self,
        operation: impl AsRef<str>,
        tag: Option<String>,
        context: Option<&ThunkContext>,
    ) -> Option<crate::prelude::Operation> {
        let operations = self.scan_root();

        let Operations(plugins, ..) = self;

        if let Some(operation) = operations.iter().find(|(label, event)| {
            let matches_operation_name = event.0 == operation.as_ref();

            if let Some(tag) = tag.as_ref() {
                label.starts_with(tag) && matches_operation_name
            } else {
                matches_operation_name
            }
        }) {
            let sequence = operation.1.sequence().expect("should have a sequence");
            return Some(plugins.start_sequence(sequence, context));
        }

        None
    }
}

impl<'a> SpecialAttribute for Operations<'a> {
    fn ident() -> &'static str {
        "operation"
    }

    fn parse(parser: &mut reality::AttributeParser, content: impl AsRef<str>) {
        Runtime::parse(parser, "");
        let world = parser.world().expect("should have world");
        let operation_entity = world.entities().create();

        if let Some(name) = parser.name() {
            if name != "operation" {
                parser.set_name(format!("{name}.operation"));
            }
        }
        parser.set_id(operation_entity.id() as u32);
        parser.define("name", Value::Symbol(content.as_ref().to_string()));
    }
}
