use std::fmt::Display;

use crate::{prelude::*, editor::General};
use reality::Block;
use specs::{Component, Entity};
use tracing::{event, Level};

/// The event component allows an entity to spawn a task for thunks, w/ a tokio runtime instance
///
#[derive(Debug, Component, Clone, Hash, PartialEq, Eq)]
#[storage(VecStorage)]
pub struct Event(
    /// Name of this event
    pub String,
    /// Thunks that will be executed,
    pub Vec<Thunk>,
    /// Sequence to execute,
    pub Option<Sequence>,
);

impl Event {
    /// Returns a new event component,
    ///
    pub fn new(block: &Block) -> Self {
        Self(block.name().to_string(), vec![], Some(Sequence::default()))
    }

    /// Returns an empty event,
    ///
    pub fn empty() -> Self {
        Self(String::default(), vec![], Some(Sequence::default()))
    }

    /// Sets the name for this event,
    ///
    pub fn set_name(&mut self, name: impl AsRef<str>) {
        self.0 = name.as_ref().to_string();
    }

    /// Adds a thunk to this event and the entity w/ it's data,
    ///
    pub fn add_thunk(&mut self, thunk: Thunk, entity: Entity) {
        if let Some(sequence) = self.2.as_mut() {
            self.1.push(thunk);
            sequence.add(entity);
        } else {
            event!(Level::WARN, "Cannot add thunk to an active event")
        }
    }

    /// Activates the event by removing the underlying sequence so that no new thunks can be added,
    ///
    /// This indicates that the sequence component on the entity is final, and no further changes
    /// will be made to it's event
    ///
    pub fn activate(&mut self) -> Option<Sequence> {
        self.2.take()
    }

    /// Reactivates the event,
    /// 
    pub fn reactivate(&mut self, sequence: Sequence) {
        self.2 = Some(sequence);
    }

    /// Returns true if this event is active, that is, the owner of this component can expect no further changes,
    ///
    pub fn is_active(&self) -> bool {
        self.sequence().is_none()
    }

    /// Returns the event's sequence,
    ///
    pub fn sequence(&self) -> Option<&Sequence> {
        self.2.as_ref()
    }

    /// Returns the event symbol
    ///
    pub fn symbol(&self) -> &String {
        &self.0
    }

    /// Creates an event component, with a task created with on_event
    /// a handle to the tokio runtime is passed to this function to customize the task spawning
    pub fn from_plugin<P>(event_name: impl AsRef<str>) -> Self
    where
        P: Plugin + ?Sized,
    {
        Self(
            event_name.as_ref().to_string(),
            vec![Thunk::from_plugin::<P>()],
            Some(Sequence::default()),
        )
    }

    /// Creates an event component from a thunk,
    ///
    pub fn from_thunk(event_name: impl AsRef<str>, thunk: Thunk) -> Self {
        Self(
            event_name.as_ref().to_string(),
            vec![thunk],
            Some(Sequence::default()),
        )
    }
}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Event `{}`", self.0)?;
        for thunk in self.1.iter() {
            writeln!(f, "\t      `{}`", thunk.0)?;
        }
        Ok(())
    }
}

impl Into<General> for &Event {
    fn into(self) -> General {
        General { 
            name: self.0.to_string(),
        }
    }
}
