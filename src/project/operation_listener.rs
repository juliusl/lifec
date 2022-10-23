use tokio::sync::mpsc::error::TryRecvError;

use crate::prelude::*;
/// System for listening for operations that need to be executed,
/// 
pub struct OperationListener<P>
where
    P: Project + From<ThunkContext>
{
    listener: P
}

impl<P> From<ThunkContext> for OperationListener<P> 
where
    P: Project + From<ThunkContext>
{
    fn from(context: ThunkContext) -> Self {
        OperationListener { listener: P::from(context) }
    }
}

impl<'a, P> System<'a> for OperationListener<P>
where
    P: Project + From<ThunkContext> 
{
    type SystemData = EventListener<'a>;

    fn run(&mut self, EventListener{ mut operations, .. }: Self::SystemData) {
        match operations.try_recv() {
            Ok(operation) => {
                self.listener.on_operation(operation)
            },
            Err(err) => match err {
                TryRecvError::Empty => {
                    // No-op
                },
                TryRecvError::Disconnected => {
                    panic!("operation channel has disconnected")
                },
            },
        }
    }
}


