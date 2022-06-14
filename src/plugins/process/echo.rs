use specs::Component;
use crate::{plugins::Plugin, RuntimeDispatcher};
use specs::storage::DenseVecStorage;

use super::Process;

#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct Echo;

impl Plugin<Process> for Echo {
    fn symbol() -> &'static str {
        "echo"
    }

    fn description() -> &'static str {
        "calls echo with text from input attribute"
    }

    fn call_with_context(context: &mut Process) {
        if let Some(input) = context.as_ref().find_text("input") {
            let command = format!("{} {}", Self::symbol(), input);
            match context.dispatch_mut(command) {
                Ok(_) => {
                    
                },
                Err(_) => {
                    
                },
            }
        }
    }
}
