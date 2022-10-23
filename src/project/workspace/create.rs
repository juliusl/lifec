use crate::prelude::*;

/// Extension trait to create the workspace,
/// 
pub trait Create {
    fn create(&self);
}

impl Create for Workspace {
    fn create(&self) {
        let work_dir = self.work_dir();

        if !work_dir.exists() {
            match std::fs::create_dir_all(work_dir) {
                Ok(_) => {
                    event!(Level::INFO, "Initialized workspace {:?}", work_dir);
                    // TODO: Initialize identity
                },
                Err(err) => {
                    event!(Level::ERROR, "Could not create workspace, {err}");
                },
            }
        }
    }
}