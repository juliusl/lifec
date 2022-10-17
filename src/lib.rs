pub use atlier::system::{combine, combine_default};
pub use atlier::system::{App, Extension, Value};
pub use plugins::{AsyncContext, SecureClient, Config, Install, Plugin, Process, Thunk, ThunkContext, Timer};
pub use specs::{
    storage::BTreeStorage, Component, DefaultVecStorage, DenseVecStorage, DispatcherBuilder,
    Entities, Entity, HashMapStorage, Join, ReadStorage, System, World, WorldExt, WriteStorage,
};

pub use reality::{
    BlockProperties, 
    Block, 
    BlockProperty, 
    BlockObject,
    BlockIndex,
    Parser,
    AttributeParser, 
    CustomAttribute,
    SpecialAttribute,
    Interpreter,
    wire::BlobDevice,
    wire::BlobSource,
    wire::ContentBroker,
    wire::MemoryBlobSource,
    wire::Sha256Digester,
};
use tracing::{event, Level};

pub mod prelude;

mod resources;
pub use resources::Resources;

// pub mod editor;
pub mod plugins;

mod state;
pub use state::AttributeGraph;
pub use state::AttributeIndex;

mod operation;
pub use operation::Operation;

mod runtime;
pub use runtime::EventSource;
pub use runtime::Runtime;

mod engine;
pub use engine::Engine;
pub use engine::Event;
pub use engine::Exit;
pub use engine::LifecycleOptions;
pub use engine::Sequence;
pub use engine::Connection;
pub use engine::Cursor;

mod host;
pub use host::Host;
pub use host::Start;
pub use host::Commands;
pub use host::Inspector;
pub use host::Sequencer;
pub use host::Executor;
pub use host::Editor;

mod project;
pub use project::Project;
pub use project::Source;
pub use project::Workspace;
pub use project::default_runtime;
pub use project::default_parser;

mod editor;
pub use editor::RuntimeEditor;

/// This function is provided by types that implement the Engine trait
pub type SetupFn = fn(&World) -> Entity;

/// This function can be provided by the config component
pub type ConfigFn = fn(&mut ThunkContext);

/// This function is generated by
pub type CreateFn = fn(&World, SetupFn, ConfigFn) -> Option<Entity>;
