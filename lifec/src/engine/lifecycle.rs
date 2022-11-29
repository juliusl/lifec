/// Enumeration of lifecycle actions to take at the end of an engine,
///
#[derive(Debug, Clone, Default)]
pub enum Lifecycle {
    /// Engine will exit on completion,
    ///
    #[default]
    Exit,
    /// Engine will start another engine on completion,
    ///
    Next,
    /// Engine will start several engines on completion,
    ///
    Fork,
    /// Engine will loop to the beginning on completion,
    ///
    Loop,
    /// Engine will repeat on completion and decrement the counter,
    ///
    /// At zero the engine will exit,
    ///
    Repeat(usize),
}
