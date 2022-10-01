use reality::{Interpreter, SpecialAttribute};
use specs::{Component, DenseVecStorage, WorldExt};

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
