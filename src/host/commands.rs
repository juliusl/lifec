use clap::Subcommand;

use crate::Start;

/// Host cli commands
/// 
#[derive(Subcommand, Clone, Debug)]
pub enum Commands {
    /// Starts the host by id or engine name
    /// 
    Start(Start), 
}
