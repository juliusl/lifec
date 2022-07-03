use std::fmt::Display;

use specs::storage::HashMapStorage;
use specs::{Component, Entity};

/// The event runtime uses this component to determine if it should execute additional events after an event completes
#[derive(Component, Debug, Default, Clone)]
#[storage(HashMapStorage)]
pub struct Sequence(
    /// sequence, a list of entities w/ events that are called in sequence
    Vec<Entity>, 
    /// cursor, if set, this entity will be called after the sequence completes
    Option<Entity>
);

impl Display for Sequence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            write!(f, "End of sequence, cursor: {:?}",  self.1)?;
        } else {
            let mut clone = self.0.clone(); 
            clone.reverse();
            for entity in clone.iter().take(1) {
                write!(f, "next: {:?} ", entity)?;
            }
            write!(f, "remaining: {} ", clone.len())?;
        }

        Ok(())
    }
}

impl From<Vec<Entity>> for Sequence {
    /// Note: Reverses the order, assuming vec was built with .push(), this is 
    /// because underneath the hood we'll pop off of this vector 
    fn from(mut vec: Vec<Entity>) -> Self {
        vec.reverse();

        Self(vec, None)
    }
}

impl Sequence {
    /// Returns true if there are no events in the sequence.
    pub fn is_empty(&self) -> bool {
        let Self(events, ..) = self;
        
        events.is_empty()
    }

    /// Adds an entity to this sequence.
    pub fn add(&mut self, entity: Entity) {
        let Self(events, ..) = self;
        events.reverse();
        events.push(entity);
        events.reverse();
    }

    /// Returns the next entity in this sequence.
    pub fn next(&mut self) -> Option<Entity> {
        let Self(events, ..) = self; 

        events.pop()
    }

    /// Returns the entity that should be called at the end of the sequence.
    pub fn cursor(&self) -> Option<Entity> {
        self.1
    }

    /// Sets the entity to dispatch at the end of the sequence, 
    /// if pointing to an entity in this sequence, setting the cursor will create a loop.
    pub fn set_cursor(&mut self, cursor: Entity) {
        self.1 = Some(cursor);
    }

    /// Takes the next event and returns a sequence with only that event
    pub fn fork(&self) -> Option<Sequence> {
        let mut clone = self.clone();

        if let Some(next) = clone.next() {
            let mut fork = Sequence::default();
            fork.add(next);
            Some(fork)
        } else {
            None 
        }
    }

    /// iterate through entities 
    pub fn iter_entities(&self) -> impl Iterator<Item = Entity> {
        let mut clone = self.0.clone();
        clone.reverse();
        clone.into_iter()
    }
}
