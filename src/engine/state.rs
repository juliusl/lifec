use std::collections::HashMap;
use std::sync::Arc;

use reality::Block;
use specs::SystemData;
use specs::prelude::*;

use crate::guest::Guest;
use crate::prelude::Appendix;
use crate::prelude::ErrorContext;
use crate::prelude::EventRuntime;
use crate::prelude::NodeCommand;
use crate::prelude::Operation;
use crate::prelude::RunmdFile;
use crate::prelude::Runtime;
use crate::prelude::Thunk;
use crate::prelude::ThunkContext;
use crate::state::AttributeGraph;

use super::Activity;
use super::Adhoc;
use super::Connection;
use super::ConnectionState;
use super::Cursor;
use super::Engine;
use super::Event;
use super::EventStatus;
use super::Limit;
use super::Performance;
use super::PluginFeatures;
use super::PluginListener;
use super::Profiler;
use super::Sequence;
use super::TickControl;
use super::Transition;
use super::Yielding;

mod engines;
mod events;
mod guests;
mod operations;
mod plugins;
mod profilers;
mod sequences;

/// Node custom command handler,
/// 
pub type NodeCommandHandler = fn(&mut State, Entity);

/// System data of all components,
/// 
#[derive(SystemData)]
pub struct State<'a> {
    /// Plugin features,
    /// 
    plugin_features: PluginFeatures<'a>,
    /// Plugin listeners,
    /// 
    plugin_listeners: PluginListener<'a>,
    /// Controls the event tick rate,
    ///
    tick_control: Write<'a, TickControl>,
    /// Appendix stores metadata on entities,
    ///
    appendix: Read<'a, Arc<Appendix>>,
    /// Channel to send error contexts,
    ///
    send_errors: Read<'a, tokio::sync::mpsc::Sender<ErrorContext>, EventRuntime>,
    /// Channel to broadcast completed plugin calls,
    ///
    send_completed: Read<'a, tokio::sync::broadcast::Sender<Entity>, EventRuntime>,
    /// Map of custom node command handlers,
    ///
    handlers: Read<'a, HashMap<String, NodeCommandHandler>>,
    /// Entity map
    ///
    entity_map: Read<'a, HashMap<String, Entity>>,
    /// Entities storage,
    /// 
    pub entities: Entities<'a>,
    /// Block storage,
    /// 
    pub blocks: WriteStorage<'a, Block>,
    /// Thunk Storage,
    /// 
    pub thunks: WriteStorage<'a, Thunk>,
    /// Adhoc Storage,
    /// 
    pub adhocs: WriteStorage<'a, Adhoc>,
    /// Limit Storage,
    /// 
    pub limits: WriteStorage<'a, Limit>,
    /// Event Storage,
    /// 
    pub events: WriteStorage<'a, Event>,
    /// Cursor Storage,
    /// 
    pub cursors: WriteStorage<'a, Cursor>,
    /// Engine Storage,
    /// 
    pub engines: WriteStorage<'a, Engine>,
    /// Guest Storage,
    /// 
    pub guests: WriteStorage<'a, Guest>,
    /// Runtime Storage,
    /// 
    pub runtimes: WriteStorage<'a, Runtime>,
    /// Sequence Storage,
    /// 
    pub sequences: WriteStorage<'a, Sequence>,
    /// Activity Storage,
    /// 
    pub activities: WriteStorage<'a, Activity>,
    /// Profiler Storage,
    /// 
    pub profilers: WriteStorage<'a, Profiler>,
    /// Connection Storage,
    /// 
    pub connections: WriteStorage<'a, Connection>,
    /// Connection state storage,
    ///
    pub connection_states: WriteStorage<'a, ConnectionState>,
    /// Operation Storage,
    /// 
    pub operations: WriteStorage<'a, Operation>,
    /// RunmdFile Storage,
    /// 
    pub runmd_files: WriteStorage<'a, RunmdFile>,
    /// Yielding Storage,
    /// 
    pub yielding: WriteStorage<'a, Yielding>,
    /// EventStatus Storage,
    /// 
    pub event_statuses: WriteStorage<'a, EventStatus>,
    /// Transition Storage,
    /// 
    pub transitions: WriteStorage<'a, Transition>,
    /// ThunkContext Storage,
    /// 
    pub contexts: WriteStorage<'a, ThunkContext>,
    /// AttributeGraph Storage,
    /// 
    pub graphs: WriteStorage<'a, AttributeGraph>,
    /// Node commands,
    /// 
    pub commands: WriteStorage<'a, NodeCommand>,
    /// Current node statuses,
    /// 
    pub samples: WriteStorage<'a, Performance>,
}

impl<'a> State<'a> {
    /// Returns mutable reference to plugin listeners,
    /// 
    pub fn plugin_listeners(&mut self) -> &mut PluginListener<'a> {
        &mut self.plugin_listeners
    }
}