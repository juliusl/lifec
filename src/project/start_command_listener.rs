use tokio::sync::mpsc::error::TryRecvError;

use crate::prelude::*;

/// System for listening to runmd,
/// 
pub struct StartCommandListener<P>
where
    P: Project + From<ThunkContext>
{
    listener: P
}

impl<P> From<ThunkContext> for StartCommandListener<P> 
where
    P: Project + From<ThunkContext>
{
    fn from(context: ThunkContext) -> Self {
        StartCommandListener { listener: P::from(context) }
    }
}

impl<'a, P> System<'a> for StartCommandListener<P>
where
    P: Project + From<ThunkContext> 
{
    type SystemData = EventListener<'a>;

    fn run(&mut self, EventListener{ mut start_commands, .. }: Self::SystemData) {
        match start_commands.try_recv() {
            Ok(start_command) => {
                self.listener.on_start_command(start_command)
            },
            Err(err) => match err {
                TryRecvError::Empty => {
                    
                },
                TryRecvError::Disconnected => {
                    panic!("start command channel has disconnected")
                },
            },
        }
    }
}

