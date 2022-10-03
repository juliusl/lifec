use clap::Args;

use crate::ThunkContext;

/// Struct for `start` command arguments
/// 
#[derive(Debug, Clone, Args)]
pub struct Start {
    /// Entity id of the event to start,
    ///
    /// The entity id can be retrieved from the print-engine-event-graph command
    ///
    #[clap(long)]
    pub id: Option<u32>,
    /// Name of engine control block to search for to start,
    /// 
    /// This will start the first event in the engine sequence,
    /// 
    #[clap(long)]
    pub engine_name: Option<String>,
    /// Optional thunk context to use to start the event,
    /// 
    /// Advanced use case,
    /// 
    #[clap(skip)]
    pub thunk_context: Option<ThunkContext>,
}