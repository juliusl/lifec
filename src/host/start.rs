use clap::Args;

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
}