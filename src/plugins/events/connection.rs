use std::fmt::Display;

use specs::{Entity, Component, DefaultVecStorage};

use super::Sequence;

/// This component configures the Sequence cursor to point at the sequence it is connected to
#[derive(Component, Debug, Default, Clone)]
#[storage(DefaultVecStorage)]
pub struct Connection(
    /// entities that are connected
    Sequence,
    /// This entity is a third-party of the connection, which owns this component.
    /// This is to make garbage collecting these connections easier.
    Option<Entity>,
);

impl From<Sequence> for Connection {
    fn from(s: Sequence) -> Self {
        Connection(s, None)
    }
}

impl Display for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let (Some(from), Some(to)) = self.connection() {
            let from = from.id();
            let to = to.id();
            write!(f, "{from} -> {to} ")?;
        }

        if let Some(owner) = self.owner() {
            let owner = owner.id();
            write!(f, "owner: {owner} ")?;
        }

        // let fork = self.2;
        // writeln!(f, " fork: {fork}")

        Ok(())
    }
}

impl Connection {
    pub fn set_owner(&mut self, owner: Entity) {
        self.1 = Some(owner);
    }

    pub fn connection(&self) -> (Option<Entity>, Option<Entity>) {
        let Self(sequence, ..) = self; 
        (sequence.last(), sequence.cursor())
    }

    pub fn owner(&self) -> Option<Entity> {
        self.1
    }
}

