use reality::{Interpreter, SpecialAttribute};
use specs::{Component, DenseVecStorage, WorldExt, ReadStorage};
use tokio::sync::oneshot;

use crate::LifecycleOptions;

/// Special attribute for engine to setup exiting on completion
///
#[derive(Default, Debug, Component)]
#[storage(DenseVecStorage)]
pub struct Exit(pub Option<()>);

impl Exit {
    /// Returns a new component,
    ///
    pub fn new() -> Self {
        Exit(Some(()))
    }

    /// Sets the component to signal exit,
    ///
    pub fn exit(&mut self) {
        self.0.take();
    }

    /// Returns true if world should exit and close,
    ///
    pub fn should_exit(&self) -> bool {
        self.0.is_none()
    }
}

impl SpecialAttribute for Exit {
    fn ident() -> &'static str {
        "exit"
    }

    fn parse(parser: &mut reality::AttributeParser, _: impl AsRef<str>) {
        parser.define("exit", true)
    }
}

impl Interpreter for Exit {
    fn initialize(&self, world: &mut specs::World) {
        world.register::<Exit>();
        world.insert(ExitListener::new());
    }

    fn interpret(&self, world: &specs::World, block: &reality::Block) {
        if block.is_control_block() {
            for index in block.index().iter().filter(|i| i.root().name() == "engine") {
                if index
                    .find_property("exit")
                    .and_then(|e| Some(e.is_enabled()))
                    .unwrap_or_default()
                {
                    let entity = world.entities().entity(block.entity());
                    world
                        .write_component()
                        .insert(entity, Exit::new())
                        .expect("should be able to insert component");
                }
            }
        }
    }
}

/// Wrapper struct over a oneshot channel,
///
pub struct ExitListener(
    pub tokio::sync::oneshot::Sender<()>,
    pub tokio::sync::oneshot::Receiver<()>,
);

impl ExitListener {
    /// Creates a new exit listener resource
    ///
    pub fn new() -> Self {
        let (tx, rx) = oneshot::channel();
        Self(tx, rx)
    }

    /// Checks to see if it should exit, if so -- signals the exit
    /// 
    pub fn check_exit(self, exits: ReadStorage<LifecycleOptions>) {
        let should_exit = exits.as_slice().iter().all(|e| match e {
            LifecycleOptions::Exit => true,
            _ => false,
        });

        if should_exit {
            self.0.send(()).ok();
        }
    }
}
