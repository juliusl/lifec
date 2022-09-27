use std::fmt::Display;

use specs::{Entity, Component, DefaultVecStorage};

use super::Sequence;

/// This component configures the Sequence cursor to point at the sequence it is connected to
#[derive(Component, Debug, Default, Clone)]
#[storage(DefaultVecStorage)]
pub struct Connection(
    /// Entities that are connected
    Sequence,
    /// This entity is a third-party of the connection the consumes this connection state,
    /// 
    /// This is to make garbage collection consumers easier
    Option<Entity>,
);

impl Connection {
    /// Sets the owner of this connection, 
    /// 
    pub fn set_owner(&mut self, owner: Entity) {
        self.1 = Some(owner);
    }

    /// Returns a tuple view of this connection,
    /// 
    pub fn connection(&self) -> (Option<Entity>, Option<Entity>) {
        let Self(sequence, ..) = self; 
        (sequence.last(), sequence.cursor())
    }

    /// Returns true if this connection is active,
    /// 
    pub fn is_connected(&self) -> bool {
        !self.0.is_empty()
    }

    /// Returns the "owner" of this connection,
    /// 
    pub fn owner(&self) -> Option<Entity> {
        self.1
    }
}

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

