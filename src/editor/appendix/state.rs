use crate::state::AttributeGraph;

/// State description,
/// 
#[derive(Clone, Default, PartialEq, Eq)]
pub struct State {
    /// Initial state,
    ///
    pub graph: AttributeGraph,
}
