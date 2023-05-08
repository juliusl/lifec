use lifec::v2::Engine;

use reality::v2::prelude::*;

/// Example of a component that uses the engine extensions,
/// 
#[derive(Runmd, Debug, Component, Clone)]
#[storage(VecStorage)]
pub struct Example {
    /// List of identifiers to entities that are "setup" event types
    /// 
    #[config(ext=engine.once)]
    setup: Vec<String>,
    /// List of identifiers to entities that are "greet" event types
    /// 
    #[config(ext=engine.start)]
    greet: Vec<String>,
    /// Extensions for managing engine behavior
    /// 
    #[ext]
    engine: Engine,
}

impl Example {
    /// Returns a new example component,
    /// 
    pub fn new() -> Self {
        Self { setup: vec![], greet: vec![], engine: Engine::new() }
    }
}