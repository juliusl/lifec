use crate::*;
use tokio::select;
use tokio::sync::oneshot;

mod network;
pub use network::NetworkEvent;
pub use network::NetworkRuntime;
pub use network::NetworkTask;
pub use network::ProxiedMessage;
pub use network::Proxy;
pub use network::BlockAddress;

mod events;
pub use events::EventRuntime;
pub use events::EventListener;

mod thunks;
pub use thunks::CancelThunk;
pub use thunks::Config;
pub use thunks::ErrorContext;
pub use thunks::StatusUpdate;
pub use thunks::Thunk;
pub use thunks::ThunkContext;
pub use thunks::SecureClient;

mod testing;
pub use testing::Test;

mod process;
pub use process::Process;

mod install;
pub use install::Install;

mod println;
pub use println::Println;

mod timer;
pub use timer::Timer;

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
    fn compile(_parser: &mut AttributeParser) {
    }

    /// Optionally, implement to execute a setup operation before the event is called
    /// 
    fn setup_operation(context: &mut ThunkContext) -> Operation {
        Operation { context: context.clone(), task: None }
    }

    /// Returns this plugin as a custom attribute, 
    /// 
    /// This allows the runmd parser to use this plugin as an attribute type,
    /// 
    fn as_custom_attr() -> CustomAttribute {
        CustomAttribute::new_with(Self::symbol(), |parser, content| {
            if let Some(world) = parser.world() {
                let child = world.entities().create();

                // This is used after .runtime
                // If the consumer writes an ident afterwards, than that will
                // be used as the event name
                let mut event_name = "call";
                if let Value::Symbol(e) = parser.value() {
                    event_name = e;
                }
                world.write_component().insert(
                    child, 
                    Event::from_plugin::<Self>(event_name)
                ).ok();

                parser.define("sequence", Value::Int(child.id() as i32));
                parser.define_child(child.clone(), Self::symbol(), Value::Symbol(content));
                parser.define_child(child.clone(), "plugin_symbol", Self::symbol());
                
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

type PluginTask = fn(&ThunkContext) -> Option<AsyncContext>;

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
pub fn combine<A, B>() -> PluginTask
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
