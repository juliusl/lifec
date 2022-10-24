pub use crate::prelude::*;
use tokio::select;
use tokio::sync::oneshot;

mod network;
pub use network::BlockAddress;
pub use network::NetworkEvent;
pub use network::NetworkRuntime;
pub use network::NetworkTask;
pub use network::ProxiedMessage;
pub use network::Proxy;
pub use network::TCP;
pub use network::UDP;

mod events;
pub use events::EventRuntime;

mod thunks;
pub use thunks::CancelThunk;
pub use thunks::Config;
pub use thunks::ErrorContext;
pub use thunks::SecureClient;
pub use thunks::StatusUpdate;
pub use thunks::Thunk;
pub use thunks::ThunkContext;

mod testing;
pub use testing::Test;

mod process;
pub use process::Process;

mod install;
pub use install::Install;

mod println;
pub use println::Println;

mod readln;
pub use readln::Readln;

mod publish;
pub use publish::Publish;

mod timer;
pub use timer::Timer;
pub use timer::TimerSettings;

mod watch;
pub use watch::Watch;

mod run;
pub use run::Run;

// mod enable_networking;
// use enable_networking::EnableNetworking;

/// Struct to archive an entity
///
#[derive(Component, Default)]
#[storage(DefaultVecStorage)]
pub struct Archive(Option<Entity>);

impl Archive {
    /// Returns the archived entity
    ///
    pub fn archived(&self) -> Option<Entity> {
        self.0
    }
}

/// Async context returned if the plugin starts an async task
pub type AsyncContext = (
    tokio::task::JoinHandle<ThunkContext>,
    tokio::sync::oneshot::Sender<()>,
);

pub struct AsThunk<P>(pub P)
where
    P: Plugin;

impl<P> Into<Thunk> for AsThunk<P>
where
    P: Plugin,
{
    fn into(self) -> Thunk {
        Thunk::from_plugin::<P>()
    }
}

/// Implement this trait to extend the events that the runtime can create
///
pub trait Plugin {
    /// Returns the symbol name representing this plugin
    ///
    fn symbol() -> &'static str;

    /// Implement to execute logic over this thunk context w/ the runtime event system,
    ///
    fn call(context: &ThunkContext) -> Option<AsyncContext>;

    /// Returns a short string description for this plugin
    ///
    fn description() -> &'static str {
        ""
    }

    /// Returns any caveats for this plugin
    ///
    fn caveats() -> &'static str {
        ""
    }

    /// Optionally, implement to customize the attribute parser,
    ///
    /// Only used if this type is being used as a CustomAttribute.
    ///
    fn compile(_parser: &mut AttributeParser) {}

    /// Returns this plugin as a custom attribute,
    ///
    /// This allows the runmd parser to use this plugin as an attribute type,
    ///
    fn as_custom_attr() -> CustomAttribute {
        CustomAttribute::new_with(Self::symbol(), |parser, content| {
            if let Some(world) = parser.world() {
                let entity = parser.entity().expect("should have an entity");
                let child = world.entities().create();
                
                // Adding the thunk to the event defines the function to call,
                {
                    event!(Level::TRACE, "Adding entity {}'s thunk to event entity {}", child.id(), entity.id());
                    let mut events = world.write_component::<Event>();
                    if let Some(event) = events.get_mut(entity) {
                        event.add_thunk(Thunk::from_plugin::<Self>(), child);
                    } else {
                        let mut event = Event::empty();
                        event.add_thunk(Thunk::from_plugin::<Self>(), child);
                        events
                            .insert(entity, event)
                            .expect("should be able to insert");
                    }
                }

                // This is used after .runtime
                // If the consumer writes an ident afterwards, than that will
                // be used as the event name
                let mut event_name = "call";
                if let Value::Symbol(e) = parser.value() {
                    event_name = e;
                }

                world.write_component()
                    .insert(child, Event::from_plugin::<Self>(event_name))
                    .ok();
                
                world.write_component()
                    .insert(child, Thunk::from_plugin::<Self>())
                    .expect("should be able to insert thunk component");

                parser.define_child(child, Self::symbol(), Value::Symbol(content));
                parser.define_child(child, "plugin_symbol", Self::symbol());
                parser.define_child(child, "event_id", entity.id() as usize);
                if !Self::description().is_empty() {
                    parser.define_child(child.clone(), "description", Self::description());
                }
                if !Self::caveats().is_empty() {
                    parser.define_child(child.clone(), "caveats", Self::caveats());
                }

                Self::compile(parser);
            }
        })
    }
}

/// Function signature for the plugin trait's call() fn
///
pub type Call = fn(&ThunkContext) -> Option<AsyncContext>;

/// Combine plugins
/// Example "Copy" plugin:
/// ```
/// use lifec::editor::Call;
/// use lifec::plugins::{OpenFile, WriteFile};
/// use lifec::Runtime;
///
/// let mut runtime = Runtime::default();
/// runtime.install::<Call, (OpenFile, WriteFile)>();
///
/// ```
pub fn combine<A, B>() -> Call
where
    A: Plugin + Default + Send,
    B: Plugin + Default + Send,
{
    <(A, B) as Plugin>::call
}

impl<A, B> Plugin for (A, B)
where
    A: Plugin + Default + Send,
    B: Plugin + Default + Send,
{
    fn symbol() -> &'static str {
        "combine"
    }

    fn description() -> &'static str {
        "Combines two plugins by calling each one by one"
    }

    fn call(context: &ThunkContext) -> Option<AsyncContext> {
        context.clone().task(|cancel_source| {
            let tc = context.clone();
            async {
                let (upper_cancel_a, cancel_source_a) = oneshot::channel::<()>();
                let (upper_cancel_b, cancel_source_b) = oneshot::channel::<()>();

                if let Some(handle) = tc.handle() {
                    let combined_task = handle.spawn(async move {
                        let mut tc = tc.clone();
                        if let Some((handle, cancel)) = A::call(&tc) {
                            select! {
                                next = handle => {
                                    match next {
                                        Ok(next) => {
                                            tc = next;
                                        },
                                        Err(err) => {
                                            event!(Level::ERROR, "error {}", err);
                                        },
                                    }
                                }
                                _ = cancel_source_a => {
                                    cancel.send(()).ok();
                                }
                            }
                        }

                        let mut next_tc = tc.commit();

                        if let Some((handle, cancel)) = B::call(&next_tc) {
                            select! {
                                next = handle => {
                                    match next {
                                        Ok(n) => {
                                            next_tc = n;
                                        },
                                        Err(err) => {
                                            event!(Level::ERROR, "error {}", err);
                                        },
                                    }
                                }
                                _ = cancel_source_b => {
                                    cancel.send(()).ok();
                                }
                            }
                        }

                        Some(next_tc)
                    });

                    return select! {
                        next = combined_task => {
                            match next {
                                Ok(next) => {
                                    next
                                },
                                _ => None
                            }
                        }
                        _ = cancel_source => {
                            upper_cancel_a.send(()).ok();
                            upper_cancel_b.send(()).ok();
                            None
                        }
                    };
                }

                None
            }
        })
    }
}
