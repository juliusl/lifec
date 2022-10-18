use std::fmt::Display;

use specs::storage::DefaultVecStorage;
use specs::{Component, Entity};

use super::Connection;

mod cursor;
pub use cursor::Cursor;

mod start_mode;
pub use start_mode::StartMode;

/// A component for a collection of entities that are processed in sequence,
/// 
/// In addition, determines the behavior to take after a sequence completes. The goal
/// of this component is to be an entity-only representation. 
/// 
/// The execution behavior is determined by the components on the entities themselves.
/// 
#[derive(Component, Debug, Default, Clone)]
#[storage(DefaultVecStorage)]
pub struct Sequence(
    /// Sequence, a list of entities w/ events that are called in sequence,
    /// 
    Vec<Entity>,
    /// Cursor, if set, this entity will be called after the sequence completes,
    /// 
    Option<Cursor>,
);

impl Display for Sequence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            write!(f, "End of sequence, cursor: {:?}", self.1)?;
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
    /// because underneath the hood we'll pop off of this vector,
    /// 
    fn from(mut vec: Vec<Entity>) -> Self {
        vec.reverse();

        Self(vec, None)
    }
}

impl Sequence {
    /// Returns true if there are no events in the sequence,
    /// 
    pub fn is_empty(&self) -> bool {
        let Self(events, ..) = self;

        events.is_empty()
    }

    /// Adds an entity to this sequence,
    /// 
    pub fn add(&mut self, entity: Entity) {
        let Self(events, ..) = self;
        events.reverse();
        events.push(entity);
        events.reverse();
    }

    /// Pushs an entity to the top of this sequence,
    /// 
    pub fn push(&mut self, entity: Entity) {
        let Self(events, ..) = self;
        events.push(entity);
    }

    /// Returns the next entity in this sequence,
    /// 
    pub fn next(&mut self) -> Option<Entity> {
        let Self(events, ..) = self;

        events.pop()
    }

    /// Returns a copy of the next entity in this sequence,
    /// w/o altering the sequence,
    /// 
    pub fn peek(&self) -> Option<Entity> {
        self.clone().0.pop()
    }

    /// Returns the copy of the last entity in this sequence, before the cursor,
    /// 
    pub fn last(&self) -> Option<Entity> {
        let mut clone = self.clone();
        clone.0.reverse();
        clone.0.pop()
    }

    /// Connects the current cursor to the start of the other sequence,
    /// by returning a sequence that contains the first entity as the the only
    /// element in the sequence, and the next entity set as the cursor,
    /// 
    pub fn connect(&self, other: &Sequence) -> Connection {
        let from = self.last();
        let to = other.peek();

        Connection {
            from,
            to,
            tracker: None,
        }
    }

    /// Resets the cursor,
    ///
    pub fn disconnect(&self) -> Self {
        let mut clone = self.clone();

        clone.1 = None;
        clone
    }

    /// Removes an entity from the cursor,
    ///
    pub fn disconnect_by(&self, entity: Entity) -> Self {
        let mut clone = self.clone();

        match &self.1 {
            Some(cursor) => match cursor {
                Cursor::Next(next) => {
                    if *next == entity {
                        clone.1 = None;
                        clone
                    } else {
                        clone
                    }
                }
                Cursor::Fork(forks) => {
                    let forks = forks
                        .iter()
                        .filter(|f| **f != entity)
                        .cloned()
                        .collect::<Vec<_>>();
                    clone.1 = Some(Cursor::Fork(forks));
                    clone
                }
                Cursor::Select(selects) => {
                    let selects = selects
                        .iter()
                        .filter(|f| **f != entity)
                        .cloned()
                        .collect::<Vec<_>>();
                    clone.1 = Some(Cursor::Select(selects));
                    clone
                }
            },
            None => clone,
        }
    }

    /// Returns the entity that should be called at the end of the sequence,
    ///
    pub fn cursor(&self) -> Option<&Cursor> {
        self.1.as_ref()
    }

    /// Sets the entity to dispatch at the end of the sequence,
    /// if pointing to an entity in this sequence, setting the cursor will create a loop,
    ///
    pub fn set_cursor(&mut self, cursor: Entity) {
        match self.1.as_mut() {
            Some(c) => match c {
                Cursor::Next(next) => self.1 = Some(Cursor::Fork(vec![*next, cursor])),
                Cursor::Fork(forks) => {
                    let mut forks = forks.clone();
                    forks.push(cursor);
                    self.1 = Some(Cursor::Fork(forks));
                }
                Cursor::Select(_) => todo!(),
            },
            None => {
                self.1 = Some(Cursor::Next(cursor));
            }
        }
    }

    /// Iterate through entities in the sequence,
    ///
    pub fn iter_entities(&self) -> impl Iterator<Item = Entity> {
        let mut clone = self.0.clone();
        clone.reverse();
        clone.into_iter()
    }
}
