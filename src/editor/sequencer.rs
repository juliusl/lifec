use std::collections::HashSet;

use specs::{Entity, System, ReadStorage, Entities, Join};

use crate::plugins::Event;

///
/// [Event, Event, Event]
/// |______________________cursor
/// tick:
///         |____________________cursor
pub struct Sequencer {
    event_index: HashSet<(String, Entity)>
}

impl<'a> System<'a> for Sequencer {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Event>
    );

    fn run(&mut self, (entities, events): Self::SystemData) {
        for (entity, event) in (&entities, events.maybe()).join() {
            if let Some(event) = event {
                self.event_index.insert((event.to_string(), entity));
            }
        }
    }
}

