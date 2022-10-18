use clap::Subcommand;

use crate::Start;

/// Host cli commands
///
#[derive(Subcommand, Clone, Debug)]
pub enum Commands {
    /// Starts the host by id or engine name
    ///
    Start(Start),
    /// Opens the editor for the host,
    ///
    Open,
}

impl Commands {
    /// Helper method to configure a start command,
    ///
    pub fn start_engine(name: impl AsRef<str>) -> Self {
        Self::Start(Start {
            engine_name: Some(name.as_ref().to_string()),
            id: None,
            thunk_context: None,
        })
    }
}
