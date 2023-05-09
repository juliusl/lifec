use reality::v2::prelude::*;

/// Component containing tokio primatives for signaling cancellation of an ongoing task,
///
#[derive(Component)]
#[storage(VecStorage)]
pub struct CancelToken {
    /// Sender to signal a cancellation of an ongoing task,
    ///
    sender: Option<tokio::sync::oneshot::Sender<()>>,
    /// Source that is listening for a cancellation signal,
    ///
    source: Option<tokio::sync::oneshot::Receiver<()>>,
}

impl CancelToken {
    /// Creates a new cancel token,
    /// 
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();

        Self { sender: Some(tx), source: Some(rx) }
    }

    /// Signals cancellation,
    /// 
    pub fn cancel(&mut self) -> Result<()> {
        if let Some(sender) = self.sender.take() {
            sender
                .send(())
                .map_err(|_| Error::new("Could not send cancel signal"))
        } else {
            Err(Error::skip())
        }
    }

    /// Activates the cancel token and returns the receiver,
    /// 
    pub fn activate(&mut self) -> Result<tokio::sync::oneshot::Receiver<()>> {
        if let Some(rx) = self.source.take() {
            Ok(rx)
        } else {
            Err(Error::new("Already activated"))
        }
    }
}

/// Trait for cancelling an ongoing task,
///
#[thunk]
pub trait Cancel {
    /// Signals cancellation for an ongoing task,
    ///
    fn cancel(&self, cancel_token: &mut CancelToken) -> Result<()>;
}
