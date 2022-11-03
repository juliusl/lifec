use specs::System;
use tracing::{event, Level};

use crate::prelude::NodeCommand;

use super::{Events, EventStatus};

/// System for cleaning up completed spawned entities,
///
#[derive(Default)]
pub struct Cleanup;

impl<'a> System<'a> for Cleanup {
    type SystemData = Events<'a>;

    fn run(&mut self, events: Self::SystemData) {
        for (spawned, _, owner) in events.iter_spawned_events() {
            match events.status(*spawned) {
                EventStatus::Completed(_) | EventStatus::Cancelled(_) => {
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
                EventStatus::Disposed(_) => {
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
