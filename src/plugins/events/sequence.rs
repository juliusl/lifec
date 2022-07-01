use specs::storage::HashMapStorage;
use specs::{Component, Entity};


#[derive(Component, Default, Clone)]
#[storage(HashMapStorage)]
pub struct Sequence(
    /// sequence, a list of entities w/ events that are called in sequence
    Vec<Entity>, 
    /// cursor, if set, this entity will be called after the sequence completes
    Option<Entity>
);

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
       
        let mut clone = events.clone();
        clone.reverse();
        clone.push(entity);
        clone.reverse();

        *events = clone;
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
}
