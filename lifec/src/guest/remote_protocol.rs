use reality::wire::Protocol;
use std::hash::Hash;

/// Struct for reading from a remote protocol,
/// 
#[derive(Clone)]
pub struct RemoteProtocol {
    /// Remote protocol being observed,
    /// 
    pub remote: tokio::sync::watch::Receiver<Protocol>,
    /// Cursor of how much of the journal has been processed,
    /// 
    journal_cursor: usize,
}

impl RemoteProtocol {
    /// Returns a new remote protocol,
    /// 
    pub fn new(remote: tokio::sync::watch::Receiver<Protocol>) -> Self {
        Self { remote, journal_cursor: 0 }
    }

    /// Advance the journal cursor,
    ///
    pub fn advance_journal_cursor(&mut self, idx: usize) {
        self.journal_cursor = idx;
    }

    /// Returns the current journal cursor,
    ///
    pub fn journal_cursor(&self) -> usize {
        self.journal_cursor
    }
}

impl Hash for RemoteProtocol {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.journal_cursor.hash(state);
    }
}

impl AsRef<tokio::sync::watch::Receiver<Protocol>> for RemoteProtocol {
    fn as_ref(&self) -> &tokio::sync::watch::Receiver<Protocol> {
        &self.remote
    }
}