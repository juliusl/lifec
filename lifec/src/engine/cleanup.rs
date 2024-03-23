use specs::{Entities, System};
use tracing::{event, Level};

use super::{EventStatus, NodeCommand, State};

/// System for cleaning up completed spawned entities,
///
#[derive(Default)]
pub struct Cleanup;

impl<'a> System<'a> for Cleanup {
    type SystemData = (Entities<'a>, State<'a>);

    fn run(&mut self, (entities, mut events): Self::SystemData) {
        let mut to_delete = vec![];
        let mut to_cleanup_connection = vec![];

        for (spawned, _, owner) in events.iter_spawned_events() {
            match events.status(*spawned) {
                EventStatus::Completed(_) | EventStatus::Cancelled(_)
                    if entities.is_alive(*spawned) =>
                {
                    to_delete.push(*spawned);
                }
                EventStatus::Disposed(_) if entities.is_alive(*owner) => {
                    to_cleanup_connection.push(*owner);
                }
                _ => {}
            }
        }

        for t in to_delete {
            events.delete(t);
        }

        for t in to_cleanup_connection {
            events.cleanup_connection(t);
        }
    }
}
