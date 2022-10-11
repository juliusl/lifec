use specs::{World, Entity, WorldExt};

use crate::{Event, Runtime, Config, Thunk, Plugin};

use tracing::{event, Level};

/// Event source returned by a runtime, that can be used to schedule events w/ a world
/// 
/// Catalogs aspects of the underlying plugin driving the event's thunk.
/// 
pub struct EventSource {
    /// The event struct component this source returns,
    /// 
    event: Event,
    /// Description of the plugin,
    /// 
    description: Option<String>, 
    /// Caveats of the plugin,
    /// 
    caveats: Option<String>, 
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
            description: {
                let description = P::description();
                if description.is_empty() {
                    None 
                } else {
                    Some(description.to_string())
                }
            },
            caveats: {
                let caveats = P::caveats();
                if caveats.is_empty() {
                    None 
                } else {
                    Some(caveats.to_string())
                }
            },
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
            description: self.description.clone(),
            caveats: self.caveats.clone(),
        }
    }
}
