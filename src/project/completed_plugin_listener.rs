use specs::System;
use tracing::event;
use tracing::Level;

use crate::{ThunkContext, Project, plugins::EventListener};

/// Listens for plugins to complete,
/// 
pub struct CompletedPluginListener<P>
where
    P: Project + From<ThunkContext>
{
    listener: P
}

impl<P> From<ThunkContext> for CompletedPluginListener<P> 
where
    P: Project + From<ThunkContext>
{
    fn from(context: ThunkContext) -> Self {
        CompletedPluginListener { listener: P::from(context) }
    }
}


impl<'a, P> System<'a> for CompletedPluginListener<P>
where
    P: Project + From<ThunkContext> 
{
    type SystemData = EventListener<'a>;

    fn run(&mut self, EventListener{ mut completed_plugins, .. }: Self::SystemData) {
        match completed_plugins.try_recv() {
            Ok(completed_entity) => {
                self.listener.on_completed_plugin_call(completed_entity)
            },
            Err(err) => match err {
                tokio::sync::broadcast::error::TryRecvError::Empty => {
                    
                },
                tokio::sync::broadcast::error::TryRecvError::Lagged(_) => {
                    
                },
                tokio::sync::broadcast::error::TryRecvError::Closed => {
                    event!(Level::INFO, "completed plugin listener is closing")
                },
            }
        }
    }
}


