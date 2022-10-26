use crate::engine::Adhoc;
use crate::prelude::{Plugins, Runtime, Sequence, ThunkContext};
use atlier::system::Value;
use reality::{Block, SpecialAttribute};
use specs::prelude::*;
use specs::{Entities, ReadStorage, SystemData, WorldExt};
use tracing::{event, Level};

/// Special attribute to define an operation in the root block for the workspace,
///
#[derive(SystemData)]
pub struct Operations<'a> {
    plugins: Plugins<'a>,
    entities: Entities<'a>,
    blocks: ReadStorage<'a, Block>,
    adhocs: ReadStorage<'a, Adhoc>,
    sequences: ReadStorage<'a, Sequence>,
}

impl<'a> Operations<'a> {
    /// Returns a map of operations and their
    ///
    pub fn scan_root(&self) -> Vec<(Adhoc, Sequence)> {
        let Operations {
            entities,
            blocks,
            adhocs,
            sequences,
            ..
        } = self;

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

                let adhoc = adhocs
                    .get(operation_entity)
                    .expect("should have an adhoc component")
                    .clone();

                let sequence = sequences
                    .get(operation_entity)
                    .expect("should have a sequence")
                    .clone();

                operations.push((adhoc, sequence));
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

        let Operations { plugins, .. } = self;

        if let Some((_, sequence)) = operations.iter().find(|(adhoc, _)| {
            let matches_operation_name = adhoc.name().as_ref() == operation.as_ref();

            if let Some(tag) = tag.as_ref() {
                adhoc.tag().as_ref() == tag && matches_operation_name
            } else {
                matches_operation_name
            }
        }) {
            return Some(plugins.start_sequence(sequence, context));
        }

        None
    }

    /// Dispatches an operation,
    ///
    pub fn dispatch_operation(
        &mut self,
        operation: impl AsRef<str>,
        tag: Option<String>,
        context: Option<&ThunkContext>,
    ) {
        let name = operation.as_ref().to_string();

        let operation = { self.execute_operation(operation, tag, context).take() };

        let Operations { plugins, .. } = self;

        match plugins
            .features()
            .broker()
            .try_send_operation(operation.expect("should have started the operation"))
        {
            Ok(_) => {
                event!(Level::DEBUG, "Dispatched operation {name}");
            }
            Err(err) => {
                event!(Level::ERROR, "Error sending operation {err}");
            }
        }
    }
}

impl<'a> SpecialAttribute for Operations<'a> {
    fn ident() -> &'static str {
        "operation"
    }

    fn parse(parser: &mut reality::AttributeParser, content: impl AsRef<str>) {
        Runtime::parse(parser, "");
        let world = parser.world().expect("should have world").clone();
        let mut adhocs = world.write_component::<Adhoc>();
        let operation_entity = world.entities().create();
        let name = content.as_ref().to_string();

        let tag = if let Some(tag) = parser.name() {
            if tag != "operation" {
                let tag = format!("{tag}.operation");
                parser.set_name(&tag);
                tag.to_string()
            } else {
                tag.to_string()
            }
        } else {
            panic!("parser should have had a name");
        };

        parser.set_id(operation_entity.id() as u32);
        parser.define("name", Value::Symbol(name.to_string()));
        adhocs
            .insert(operation_entity, Adhoc { name, tag })
            .expect("should be able to insert component");
    }
}
