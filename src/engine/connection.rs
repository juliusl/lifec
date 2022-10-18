use std::fmt::Display;

use specs::{Entity, Component, DefaultVecStorage};

/// This component configures the Sequence cursor to point at the sequence it is connected to
/// 
#[derive(Component, Debug, Default, Clone)]
#[storage(DefaultVecStorage)]
pub struct Connection {
    pub from: Option<Entity>, 
    pub to: Option<Entity>, 
    pub tracker: Option<Entity>,
}

impl Connection {
    /// Sets the owner of this connection, 
    /// 
    pub fn set_owner(&mut self, owner: Entity) {
        self.tracker = Some(owner);
    }

    /// Returns a tuple view of this connection,
    /// 
    pub fn connection(&self) -> (Option<Entity>, Option<Entity>) {
        (self.from, self.to)
    }

    /// Returns true if this connection is active,
    /// 
    pub fn is_connected(&self) -> bool {
        self.from.is_some() && self.to.is_some()
    }

    /// Returns the "owner" of this connection,
    /// 
    pub fn owner(&self) -> Option<Entity> {
        self.tracker
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

