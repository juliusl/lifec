use std::fmt::Display;

use crate::{Config, Plugin, Thunk, ThunkContext, SpecialAttribute, AttributeParser};
use atlier::system::Value;
use specs::{Component, DenseVecStorage};
use tokio::task::JoinHandle;
use tracing::event;
use tracing::Level;

/// The event component allows an entity to spawn a task for thunks, w/ a tokio runtime instance
/// 
#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct Event(
    /// Name of this event
    pub String,
    /// Thunk that is being executed
    pub Thunk,
    /// Config for the thunk context before being executed
    pub Option<Config>,
    /// Initial context that starts this event
    pub Option<ThunkContext>,
    /// This is the task that
    pub Option<JoinHandle<ThunkContext>>,
);

impl Event {
    /// Returns the event symbol
    ///
    pub fn symbol(&self) -> &String {
        &self.0
    }

    /// Returns the a clone of the inner thunk
    ///
    pub fn thunk(&self) -> Thunk {
        self.1.clone()
    }

    /// Creates an event component, with a task created with on_event
    /// a handle to the tokio runtime is passed to this function to customize the task spawning
    pub fn from_plugin<P>(event_name: impl AsRef<str>) -> Self
    where
        P: Plugin + ?Sized,
    {
        Self(
            event_name.as_ref().to_string(),
            Thunk::from_plugin::<P>(),
            None,
            None,
            None,
        )
    }

    /// Sets the config to use w/ this event
    pub fn set_config(&mut self, config: Config) {
        self.2 = Some(config);
    }

    /// Prepares an event for the event runtime to start, cancel any previous join_handle
    ///
    /// Caveats: If the event has a config set, it will configure the context, before setting it
    ///
    pub fn fire(&mut self, thunk_context: ThunkContext) {
        self.3 = Some(thunk_context);

        // cancel any current task
        self.cancel();
    }

    /// If a config is set w/ this event, this will setup a thunk context
    /// from that config. Otherwise, No-OP.
    pub fn setup(&self, thunk_context: &mut ThunkContext) {
        if let Some(Config(name, config)) = self.2 {
            event!(
                Level::TRACE,
                "detected config '{name}' for event: {} {}",
                self.symbol(),
                self.1.symbol()
            );
            config(thunk_context);
        }
    }

    /// Cancel the existing join handle, mainly used for housekeeping.
    /// Thunks must manage their own cancellation by using the cancel_source.
    pub fn cancel(&mut self) {
        if let Some(task) = self.4.as_mut() {
            task.abort();
        }
    }

    /// returns true if task is running
    ///
    pub fn is_running(&self) -> bool {
        self.4
            .as_ref()
            .and_then(|j| Some(!j.is_finished()))
            .unwrap_or_default()
    }

    /// Creates a duplicate of this event
    ///
    pub fn duplicate(&self) -> Self {
        Self(
            self.0.to_string(),
            self.1.clone(),
            self.2.clone(),
            None,
            None,
        )
    }
}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ", self.0)?;
        write!(f, "{}", self.1 .0)?;
        Ok(())
    }
}

impl SpecialAttribute for Event {
    fn ident() -> &'static str {
        "event"
    }

    fn parse(parser: &mut AttributeParser, content: impl AsRef<str>) {
        let idents = Event::parse_idents(content.as_ref());

        match (idents.get(0), idents.get(1)) {
            (Some(name), Some(symbol)) => {
                parser.define("event", Value::Symbol(format!("{name} {symbol}")));
            },
            (Some(symbol), None) => {
                parser.define("event", Value::Symbol(symbol.to_string()));
            },
            _ => {
                event!(Level::ERROR, "Invalid format idents state");
            }
        }
    }
}
