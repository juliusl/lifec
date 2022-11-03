use specs::Join;
use tracing::{event, Level};

use crate::{prelude::{Sequence, ThunkContext}, engine::Adhoc};

use super::State;

impl<'a> State<'a> {
    /// Returns a map of operations and their
    ///
    pub fn find_root_operations(&self) -> Vec<(Adhoc, Sequence)> {
        let Self {
            entities,
            blocks,
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

                if let Some((adhoc, operation)) = (&self.adhocs, &self.sequences).join().get(operation_entity, entities) {
                    operations.push((adhoc.clone(), operation.clone()));
                }
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
        let operations = self.find_root_operations();

        if let Some((_, sequence)) = operations.iter().find(|(adhoc, _)| {
            let matches_operation_name = adhoc.name().as_ref() == operation.as_ref();

            if let Some(tag) = tag.as_ref() {
                adhoc.tag().as_ref() == tag && matches_operation_name
            } else {
                matches_operation_name
            }
        }) {
            return Some(self.start_sequence(sequence, context));
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


        match self
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