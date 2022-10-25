use crate::prelude::{Listener, Engine};

/// Resource to collect dispatched messages,
///  
#[derive(Default)]
pub struct Runner;

impl Listener for Runner {
    fn create(world: &specs::World) -> Self {
        if let Some(_runner) = Engine::find_block(world, "runner") {
        }
        Self::default()
    }

    fn on_runmd(&mut self, runmd: &crate::prelude::RunmdFile) {
        if runmd.symbol == "job" {

        }
    }

    fn on_operation(&mut self, operation: crate::prelude::Operation) {
        todo!()
    }

    fn on_start_command(&mut self, start_command: &super::Start) {
        todo!()
    }

    fn on_status_update(&mut self, status_update: &crate::prelude::StatusUpdate) {
        todo!()
    }

    fn on_completed_event(&mut self, entity: &specs::Entity) {
        todo!()
    }

    fn on_error_context(&mut self, error: &crate::prelude::ErrorContext) {
        todo!()
    }
}