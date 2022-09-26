
use crate::*;
use tokio::select;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

mod secure;
pub use secure::Secure;

mod block;
pub use block::BlockAddress;
pub use block::BlockContext;
pub use block::Project;

mod network;
pub use network::NetworkEvent;
pub use network::NetworkRuntime;
pub use network::NetworkTask;
pub use network::ProxiedMessage;
pub use network::Proxy;

mod events;
pub use events::Event;
pub use events::Sequence;
pub use events::Connection;
pub use events::EventRuntime;
pub use events::ProxyDispatcher;

mod process;
// pub use process::Expect;
// pub use process::Missing;
// pub use process::Remote;
pub use process::Process;
pub use process::Redirect;
pub use process::Install;

mod thunks;
pub use thunks::CancelThunk;
pub use thunks::Config;
pub use thunks::Dispatch;
pub use thunks::ErrorContext;
pub use thunks::StatusUpdate;
pub use thunks::Thunk;
pub use thunks::ThunkContext;
pub use thunks::Timer;
pub use thunks::WriteFile;

mod testing;
pub use testing::Test;

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

    /// Optionally, implement to customize the attribute parser
    /// 
    fn customize(_parser: &mut AttributeParser) {
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
        reality::CustomAttribute::new_with(Self::symbol(), |parser, content| {
            if let Some(world) = parser.world() {
                let child = world.entities().create();

                parser.define_child(child, Self::symbol(), Value::Symbol(content));

                parser.define("sequence", Value::Int(child.id() as i32));
            
                Self::customize(parser);
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
                        if let Some((handle, cancel)) = A::call(&mut tc) {
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

                        // let previous = tc
                        //     .project
                        //     .as_ref()
                        //     .and_then(|p| p.transpile_blocks().ok())
                        //     .unwrap_or_default()
                        //     .trim()
                        //     .to_string();

                        let mut next_tc = tc.clone();
                        // if !previous.trim().is_empty() {
                        //     let block_name = tc.block.name.unwrap().to_string();
                        //     next_tc
                        //         .as_mut()
                        //         .add_message(block_name, "previous", previous);
                        // }

                        if let Some((handle, cancel)) = B::call(&mut next_tc) {
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
