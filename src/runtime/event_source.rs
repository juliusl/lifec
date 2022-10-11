use specs::{World, Entity, WorldExt};

use crate::{Event, Runtime, Config, Thunk, Plugin};

use tracing::{event, Level};

/// Event source returned by a runtime, that can be used to schedule events w/ a world
/// 
pub struct EventSource {
    /// The event struct component this source returns 
    /// 
    event: Event,
}

impl EventSource {
    /// Returns a new event source,
    /// 
    pub fn new<P>(event_name: impl AsRef<str>) -> Self 
    where 
        P: Plugin + Default + Send
    {
        EventSource {
            event: Event::from_plugin::<P>(event_name),
        }
    }

    /// Sets the config for the event,
    /// 
    pub fn set_config(&mut self, config: Config) {
        self.event.set_config(config);
    }

    /// Creates a new entity w/ the underlying event,
    /// 
    pub fn create_entity(&self, world: &World) -> Option<Entity> {
        let entity = world.entities().create();

        match world.write_component().insert(entity, self.event.duplicate()) {
            Ok(_) => {
                event!(Level::DEBUG, "Creating a new entity for event {}", self.event);
                Some(entity)
            },
            Err(err) => {
                event!(Level::ERROR, "Error inserting event, {err}");
                None
            },
        }
    }

    /// Returns the event's plugin thunk
    /// 
    pub fn thunk(&self) -> Thunk {
        self.event.thunk()
    }
}

impl Clone for EventSource {
    fn clone(&self) -> Self {
        Self { 
            event: self.event.duplicate(),
        }
    }
}
