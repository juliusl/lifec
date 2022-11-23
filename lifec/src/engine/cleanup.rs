use specs::{System, Entities};
use tracing::{event, Level};

use super::{State, EventStatus, NodeCommand};

/// System for cleaning up completed spawned entities,
///
#[derive(Default)]
pub struct Cleanup;

impl<'a> System<'a> for Cleanup {
    type SystemData = (Entities<'a>, State<'a>);

    fn run(&mut self, (entities, events): Self::SystemData) {
        for (spawned, _, owner) in events.iter_spawned_events() {
            match events.status(*spawned) {
                EventStatus::Completed(_) | EventStatus::Cancelled(_) if entities.is_alive(*spawned) => {
                    match events.plugins().features().broker().try_send_node_command(
                        NodeCommand::custom("delete_spawned", *spawned),
                        None,
                    ) {
                        Ok(_) => {
                            event!(Level::DEBUG, "Deleting spawned, {}", spawned.id());
                        }
                        Err(err) => {
                            event!(Level::ERROR, "Could not send node command, {err}");
                        }
                    }
                }
                EventStatus::Disposed(_) if entities.is_alive(*owner)  => {
                    match events.plugins().features().broker().try_send_node_command(
                        NodeCommand::custom("cleanup_connection", *owner),
                        None,
                    ) {
                        Ok(_) => {
                            event!(Level::DEBUG, "Deleting spawned, {}", spawned.id());
                        }
                        Err(err) => {
                            event!(Level::ERROR, "Could not send node command, {err}");
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
