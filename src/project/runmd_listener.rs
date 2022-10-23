use tokio::sync::mpsc::error::TryRecvError;

use crate::prelude::*;

/// System for listening to runmd,
/// 
pub struct RunmdListener<P>
where
    P: Project + From<ThunkContext>
{
    listener: P
}

impl<P> From<ThunkContext> for RunmdListener<P> 
where
    P: Project + From<ThunkContext>
{
    fn from(context: ThunkContext) -> Self {
        RunmdListener { listener: P::from(context) }
    }
}

impl<'a, P> System<'a> for RunmdListener<P>
where
    P: Project + From<ThunkContext> 
{
    type SystemData = EventListener<'a>;

    fn run(&mut self, EventListener{ mut runmd, .. }: Self::SystemData) {
        match runmd.try_recv() {
            Ok(runmd) => {
                self.listener.on_runmd(runmd)
            },
            Err(err) => match err {
                TryRecvError::Empty => {
                    
                },
                TryRecvError::Disconnected => {
                    panic!("runmd channel has disconnected")
                },
            },
        }
    }
}

