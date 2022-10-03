use specs::System;
use tokio::sync::mpsc::error::TryRecvError;

use crate::{ThunkContext, Project, plugins::EventListener};

/// System for listening for errors,
/// 
pub struct ErrorContextListener<P>
where
    P: Project + From<ThunkContext>
{
    listener: P
}

impl<P> From<ThunkContext> for ErrorContextListener<P> 
where
    P: Project + From<ThunkContext>
{
    fn from(context: ThunkContext) -> Self {
        ErrorContextListener { listener: P::from(context) }
    }
}

impl<'a, P> System<'a> for ErrorContextListener<P>
where
    P: Project + From<ThunkContext> 
{
    type SystemData = EventListener<'a>;

    fn run(&mut self, EventListener{ mut error_contexts, .. }: Self::SystemData) {
        match error_contexts.try_recv() {
            Ok(error_context) => {
                self.listener.on_error_context(error_context)
            },
            Err(err) => match err {
                TryRecvError::Empty => {
                    // No-op
                },
                TryRecvError::Disconnected => {
                    panic!("error context channel has disconnected")
                },
            },
        }
    }
}


