use specs::{Component, DenseVecStorage};

/// Enumeration of transition strategy between events,
/// 
/// # Example usage, defining an engine sequence
/// 
/// First, a `runner` engine is defined,
/// 
/// + .engine
/// : .once   setup
/// : .start  receive, cancel
/// : .select execute
/// : .start  complete
/// : .fork   operation, runner <Names of the next engines to start> 
/// 
/// elsewhere, an `operation` engine is defined,
/// 
/// + .engine
/// : .start  format
/// : .spawn  process
/// : .buffer record
/// : .exit   
/// 
/// 
#[derive(Component, Default, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[storage(DenseVecStorage)]
pub enum Transition {
    /// Default transition, cancel any ongoing tasks and replace w/ the incoming task
    /// 
    #[default]
    Start,
    /// Takes one transition, afterwards skips execution and proceeds to the next transition,
    /// 
    Once,
    /// Instead of cancelling the ongoing task, starts a new branch
    /// 
    Spawn,
    /// Take multiple events, pick first that completes
    /// 
    Select,
    /// Buffer incoming transitions,
    /// 
    Buffer,
}
