use reality::SpecialAttribute;

/// Special attribute for engine to setup exiting on completion
/// 
#[derive(Debug)]
pub struct Exit(tokio::sync::oneshot::Sender<()>);

/// Wrapper struct over oneshot receiver
/// 
pub struct ExitListener(pub tokio::sync::oneshot::Receiver<()>);

impl Exit {
    /// Returns a new component,
    /// 
    pub fn new() -> (Self, ExitListener) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        (Self(tx), ExitListener(rx))
    }

    /// Sets the component to signal exit,
    /// 
    pub async fn exit(self) {
        self.0.send(()).ok();
    }

    /// Returns true if world should exit and close,
    /// 
    pub fn should_exit(&self) -> bool {
        self.0.is_closed()
    }
}

impl SpecialAttribute for Exit {
    fn ident() -> &'static str {
        "exit"
    }

    fn parse(parser: &mut reality::AttributeParser, _: impl AsRef<str>) {
        parser.define("exit_on_completion", true)
    }
}