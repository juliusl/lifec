use crate::ThunkContext;



/// The start mode controls how a sequence starts after being activated by a cursor,
/// 
#[derive(Clone, Debug)]
pub enum StartMode {
    /// Starts only once (non-reentrant), 
    /// 
    Once(ThunkContext),
    /// Cancel any ongoing task and begin immediately,
    /// 
    Immediate(ThunkContext),
    /// Buffer the transition,
    /// 
    Buffer(Vec<ThunkContext>),
}