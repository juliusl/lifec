use specs::{World, Entity};

use crate::{Event, Runtime, Config, Thunk, Plugin};


/// Event source returned by a runtime, that can be used to schedule events w/ a world
/// 
pub struct EventSource {
    /// The event struct component this source returns 
    /// 
    event: Event, 
    /// The runtime that created this event source 
    /// 
    runtime: Runtime, 
}

impl EventSource {
    /// Returns a new event source
    /// 
    pub fn new<P>(runtime: Runtime, event_name: &'static str) -> Self 
    where 
        P: Plugin + Default + Send
    {
        EventSource {
            event: Event::from_plugin::<P>(event_name),
            runtime,
        }
    }

    /// Sets the config for the event
    /// 
    pub fn set_config(&mut self, config: Config) {
        self.event.set_config(config);
    }

    /// Creates a new entity w/ this event 
    /// 
    pub fn create_entity(&self, world: &World) -> Option<Entity> {
       None
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
            runtime: self.runtime.clone(),
        }
    }
}
