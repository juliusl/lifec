use tokio::sync::mpsc::error::TryRecvError;

use crate::prelude::*;

/// System for listening to status updates,
/// 
pub struct StatusUpdateListener<P>
where
    P: Project + From<ThunkContext>
{
    listener: P
}

impl<P> From<ThunkContext> for StatusUpdateListener<P> 
where
    P: Project + From<ThunkContext>
{
    fn from(context: ThunkContext) -> Self {
        StatusUpdateListener { listener: P::from(context) }
    }
}

impl<'a, P> System<'a> for StatusUpdateListener<P>
where
    P: Project + From<ThunkContext> 
{
    type SystemData = EventListener<'a>;

    fn run(&mut self, EventListener{ mut status_updates, .. }: Self::SystemData) {
        match status_updates.try_recv() {
            Ok(status_update) => {
                self.listener.on_status_update(status_update)
            },
            Err(err) => match err {
                TryRecvError::Empty => {
                    // No-op
                },
                TryRecvError::Disconnected => {
                    panic!("status update channel has disconnected")
                },
            },
        }
    }
}


