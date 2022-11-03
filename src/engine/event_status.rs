use std::fmt::Display;

use specs::{Component, Entity, DenseVecStorage};


/// Enumeration of event statuses,
///
#[derive(Component, Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[storage(DenseVecStorage)]
pub enum EventStatus {
    /// Means that the operation is empty has no activity
    ///
    Scheduled(Entity),
    /// Means that a new operation is required
    ///
    New(Entity),
    /// Means that the operation is in progress
    ///
    InProgress(Entity),
    /// Means that the entity has been paused
    ///
    Paused(Entity),
    /// Means that the operation is ready to transition
    ///
    Ready(Entity),
    /// Means that the operation has already completed
    ///
    Completed(Entity),
    /// Means that the operation has already completed
    ///
    Cancelled(Entity),
    /// Means that the event has not been activated yet
    ///
    Inactive(Entity),
}

impl EventStatus {
    /// Returns the entity,
    ///
    pub fn entity(&self) -> Entity {
        match self {
            EventStatus::Scheduled(e)
            | EventStatus::Paused(e)
            | EventStatus::New(e)
            | EventStatus::InProgress(e)
            | EventStatus::Ready(e)
            | EventStatus::Completed(e)
            | EventStatus::Cancelled(e)
            | EventStatus::Inactive(e) => *e,
        }
    }
}

impl Display for EventStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventStatus::Scheduled(_) => write!(f, "scheduled"),
            EventStatus::New(_) => write!(f, "new"),
            EventStatus::InProgress(_) => write!(f, "in progress"),
            EventStatus::Ready(_) => write!(f, "ready"),
            EventStatus::Completed(_) => write!(f, "completed"),
            EventStatus::Cancelled(_) => write!(f, "cancelled"),
            EventStatus::Inactive(_) => write!(f, "inactive"),
            EventStatus::Paused(_) => write!(f, "paused"),
        }
    }
}