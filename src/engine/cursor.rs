use std::fmt::Display;

use specs::{Component, Entity, VecStorage};

/// Enumeration of cursor types for a sequence,
///
#[derive(Component, Debug, Clone, Hash, PartialEq, Eq)]
#[storage(VecStorage)]
pub enum Cursor {
    /// Cursor that points to one other entity,
    ///
    Next(Entity),
    /// Cursor that points to many entities,
    ///
    Fork(Vec<Entity>),
}

impl Display for Cursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cursor::Next(next) => write!(f, "-> {:02}", next.id()),
            Cursor::Fork(forks) => {
                write!(
                    f,
                    "-> {}",
                    forks
                        .iter()
                        .map(|f| format!("{:02}", f.id()))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    }
}
