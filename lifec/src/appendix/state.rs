use crate::state::AttributeGraph;

/// State description,
///
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Config {
    /// Control symbol,
    ///
    pub control_symbol: String,
    /// Initial state,
    ///
    pub graph: Option<AttributeGraph>,
}
