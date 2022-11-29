use specs::{Component, VecStorage};

/// Component to distinguish adhoc workspace operations,
///
#[derive(Component, Clone, Hash, PartialEq, Eq)]
#[storage(VecStorage)]
pub struct Adhoc {
    /// Name of the adhoc component
    ///
    pub name: String,
    /// Tag config,
    ///
    pub tag: String,
}

impl Adhoc {
    /// Returns the tag name,
    ///
    pub fn tag(&self) -> impl AsRef<str> + '_ {
        self.tag.trim_end_matches("operation").trim_end_matches(".")
    }

    /// Returns the name of the adhoc
    ///
    pub fn name(&self) -> impl AsRef<str> + '_ {
        &self.name
    }
}
