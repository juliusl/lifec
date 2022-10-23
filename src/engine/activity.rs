use std::time::Instant;

use specs::{Component, DenseVecStorage};

use crate::prelude::ErrorContext;

/// Component to track activity state changes,
///
#[derive(Debug, Clone, Component)]
#[storage(DenseVecStorage)]
pub enum Activity {
    /// Scheduled to run,
    ///
    Scheduled(Instant),
    /// Skipped
    /// 
    Skipped(Instant, Instant),
    /// Started to run,
    ///
    Started(Instant, Instant, usize),
    /// Completed run,
    ///
    Completed(Instant, Instant, Instant, usize),
    /// Ran into an error while running,
    ///
    Error(Instant, Instant, Instant, usize),
    /// No activity
    ///
    None,
}

impl Activity {
    /// Create a new activity
    ///
    pub fn schedule() -> Self {
        Activity::Scheduled(Instant::now())
    }

    /// Transitions a scheduled or error'd activity else no-op,
    ///
    pub fn start(&self) -> Option<Self> {
        match &self {
            Activity::Scheduled(scheduled) => {
                Some(Activity::Started(*scheduled, Instant::now(), 1))
            }
            Activity::Skipped(scheduled, _) => Some(Activity::Started(*scheduled, Instant::now(), 1)),
            Activity::Started(_, _, _) => None,
            Activity::Completed(scheduled, _, _, iterations) => Some(Activity::Started(
                *scheduled,
                Instant::now(),
                *iterations + 1,
            )),
            Activity::Error(scheduled, _, _, iterations) => Some(Activity::Started(
                *scheduled,
                Instant::now(),
                *iterations + 1,
            )),
            Activity::None => None,
        }
    }

    /// Transitions a started activity, if an error context exists, transitions to Error,
    ///
    /// Otherwise, transitions to Completed, else no-op
    ///
    pub fn complete(&self, error_context: Option<&ErrorContext>) -> Self {
        match &self {
            Activity::Scheduled(scheduled) => Activity::Skipped(*scheduled, Instant::now()),
            Activity::Skipped(..) => self.clone(),
            Activity::Started(scheduled, started, iterations) if error_context.is_some() => {
                Activity::Error(*scheduled, *started, Instant::now(), *iterations)
            }
            Activity::Started(scheduled, started, iterations) => {
                Activity::Completed(*scheduled, *started, Instant::now(), *iterations)
            }
            Activity::Completed(..) => self.clone(),
            Activity::Error(..) => self.clone(),
            Activity::None => Activity::None,
        }
    }
}

impl Default for Activity {
    fn default() -> Self {
        Self::None
    }
}
