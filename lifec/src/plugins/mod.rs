use std::collections::BTreeMap;
use std::collections::HashMap;
use std::ops::Deref;

pub use crate::prelude::*;
use reality::wire::Interner;
use reality::wire::ResourceId;
use tokio::sync::oneshot;

// mod network;
// pub use network::BlockAddress;
// pub use network::NetworkEvent;
// pub use network::NetworkRuntime;
// pub use network::NetworkTask;
// pub use network::ProxiedMessage;
// pub use network::Proxy;
// pub use network::TCP;
// pub use network::UDP;

pub mod attributes;

mod events;
pub use events::EventRuntime;
pub use events::Journal;

mod thunks;
pub use thunks::ErrorContext;
pub use thunks::SecureClient;
pub use thunks::StatusUpdate;
pub use thunks::Thunk;
pub use thunks::ThunkContext;

mod testing;
pub use testing::Chaos;
pub use testing::Test;

mod process;
pub use process::Process;

// mod remote;
// pub use remote::Remote;

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
use tokio::sync::oneshot::Receiver;
pub use watch::Watch;

mod run;
pub use run::Run;

mod request;
pub use request::Request;

mod define;
pub use define::Define;

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

/// Type alias for a context handler function,
///
pub type ContextHandler = fn(ThunkContext) -> Option<ThunkContext>;

/// Helper function for awaiting a plugin call from within a plugin call,
///
pub async fn await_plugin<P>(
    cancel_source: Receiver<()>,
    context: &mut ThunkContext,
    on_result: ContextHandler,
) -> Option<ThunkContext>
where
    P: Plugin,
{
    let mut clone = context.clone();
    if let Some((task, cancel)) = P::call(context) {
        select! {
            result = task => {
                match result {
                    Ok(context) => {
                        event!(Level::TRACE, "Plugin task completed");
                        on_result(context)
                    },
                    Err(err) => {
                        event!(Level::ERROR, "Error awaiting plugin call, {err}");
                        clone.error(|graph| {
                            graph.with_text("error", format!("{err}"));
                        });
                        Some(clone)
                    },
                }
            },
            _ = cancel_source => {
                cancel.send(()).ok();
                event!(Level::WARN, "Cancelling plugin call");
                None
            }
        }
    } else {
        event!(Level::WARN, "Plugin did not return a task");
        None
    }
}

/// Implement this trait to extend the events that the runtime can create
///
pub trait Plugin {
    /// Returns the symbol custom_attr representing this plugin
    ///
    fn symbol() -> &'static str;

    /// Implement to execute logic over this thunk context w/ the runtime event system,
    ///
    fn call(context: &mut ThunkContext) -> Option<AsyncContext>;

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

    /// Starts a documentation token to build documentation during compile(),
    ///
    fn start_docs<'a>(parser: &'a mut AttributeParser) -> Option<DocumentationToken<'a>> {
        if let Some(_) = parser.world() {
            let mut interner = Interner::default();
            let base_ident_id = interner.add_ident(Self::symbol());
            Some(DocumentationToken {
                base_ident_id,
                parser,
                interner,
                docs: HashMap::default(),
            })
        } else {
            None
        }
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
    fn as_custom_attr() -> CustomAttribute
    where
        Self: Sized + BlockObject,
    {
        CustomAttribute::new_with(Self::symbol(), |parser, content| {
            if let Some(world) = parser.world() {
                let entity = parser.entity().expect("should have an entity");
                let child = world.entities().create();

                // Adding the thunk to the event defines the function to call,
                {
                    event!(
                        Level::TRACE,
                        "Adding entity {}'s thunk to event entity {}",
                        child.id(),
                        entity.id()
                    );
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

                world
                    .write_component()
                    .insert(child, Thunk::from_plugin::<Self>())
                    .expect("should be able to insert thunk component");

                if let Some(label) = parser.symbol() {
                    parser.define_child(child, "label", Value::Symbol(label.to_string()));
                }

                parser.define_child(child, "event_id", Value::Int(entity.id() as i32));
                parser.define_child(child, Self::symbol(), Value::Symbol(content));

                Self::compile(parser);
            }
        })
    }
}

/// Finds documentation for an attribute related to this plugin,
///
pub fn find_doc(
    plugin: impl AsRef<str>,
    custom_attr: impl AsRef<str>,
    world: &World,
) -> Option<Documentation> {
    let mut interner = Interner::default();
    let id = interner.add_ident(plugin);
    let id = id | interner.add_ident(custom_attr);
    let resource_id = ResourceId::new_with_dynamic_id::<Documentation>(id);

    world
        .try_fetch_by_id::<Documentation>(resource_id)
        .and_then(|d| Some(d.deref().clone()))
}

/// Returns all documentation for this plugin,
///
pub fn docs(plugin: impl AsRef<str>, world: &World) -> BTreeMap<(String, u64), Documentation> {
    let mut docs = BTreeMap::default();

    if let Some(mut interner) = find_doc_interner(plugin.as_ref(), world) {
        let base_id = interner.add_ident(plugin.as_ref());
        for (id, attr) in interner.strings().iter().filter(|(i, _)| **i != base_id) {
            let resource_id = ResourceId::new_with_dynamic_id::<Documentation>(id ^ base_id);

            if let Some(doc) = world.try_fetch_by_id::<Documentation>(resource_id) {
                docs.insert((attr.clone(), id ^ base_id), doc.deref().clone());
            }
        }
    }

    docs
}

/// Finds the doc interner for a plugin,
///
pub fn find_doc_interner(plugin: impl AsRef<str>, world: &World) -> Option<Interner> {
    let mut interner = Interner::default();
    let base_id = interner.add_ident(plugin);
    let resource_id = ResourceId::new_with_dynamic_id::<Interner>(base_id);

    if let Some(_interner) = world.try_fetch_by_id::<Interner>(resource_id) {
        interner = interner.merge(&_interner);
        Some(interner)
    } else {
        None
    }
}

/// Implement trait to add documentation,
///
pub trait AddDoc<'a, 'b> {
    /// Adds documentation,
    ///
    fn add_doc(
        &'a self,
        token: &'a mut DocumentationToken<'b>,
        summary: impl AsRef<str>,
    ) -> &mut Documentation;
}

impl<'a, 'b> AddDoc<'a, 'b> for CustomAttribute {
    fn add_doc(
        &'a self,
        token: &'a mut DocumentationToken<'b>,
        summary: impl AsRef<str>,
    ) -> &mut Documentation {
        let mut doc: Documentation = summary.as_ref().into();
        doc.custom_attr().input();
        token.add_doc(self.ident(), doc)
    }
}

/// Wrapper struct for constructing documentation during `fn compile(..)`,
///
/// When this struct is dropped, the documentation will be lazily added to the World
///
pub struct DocumentationToken<'a> {
    /// Base id,
    ///
    base_ident_id: u64,
    /// Current parser being used,
    ///
    parser: &'a mut AttributeParser,
    /// Interner for converting strings into id's
    ///
    interner: Interner,
    /// Map of docs being added,
    ///
    docs: HashMap<ResourceId, Documentation>,
}

impl<'a> DocumentationToken<'a> {
    /// Adds documentation for an attribute,
    ///
    pub fn add_doc(
        &mut self,
        attr_name: impl AsRef<str>,
        doc: impl Into<Documentation>,
    ) -> &mut Documentation {
        let custom_attr_id = self.interner.add_ident(attr_name.as_ref());
        let resource_id =
            ResourceId::new_with_dynamic_id::<Documentation>(self.base_ident_id ^ custom_attr_id);

        self.docs.insert(resource_id.clone(), doc.into());
        self.docs
            .get_mut(&resource_id)
            .expect("should exist, just added")
    }
}

impl<'a> Drop for DocumentationToken<'a> {
    fn drop(&mut self) {
        let mut docs = self.docs.clone();
        let base_id = self.base_ident_id;
        let interner = self.interner.clone();
        self.parser.lazy_exec_mut(move |world| {
            for (r, d) in docs.drain() {
                world.insert_by_id(r, d);
            }

            let interner_resource_id = ResourceId::new_with_dynamic_id::<Interner>(base_id);
            world.insert_by_id(interner_resource_id, interner);
        });
    }
}

impl<'a> AsMut<AttributeParser> for DocumentationToken<'a> {
    fn as_mut(&mut self) -> &mut AttributeParser {
        self.parser
    }
}

/// Function signature for the plugin trait's call() fn
///
pub type Call = fn(&mut ThunkContext) -> Option<AsyncContext>;

/// Type alias for the Plugin trait's compile fn
///
pub type Compile = fn(&mut AttributeParser);

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

    fn call(context: &mut ThunkContext) -> Option<AsyncContext> {
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

                        let mut next_tc = tc.consume();

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

/// Prelude for building protocol implementations and plugins,
///
pub mod protocol_prelude {
    /// Helpers for writing concrete implementations for each stream required by the protocol,
    ///
    pub use crate::plugins::async_protocol_helpers::*;
    /// Wire protocol that defines the concept of a "wire object",
    ///
    pub use reality::wire::Protocol;
}

/// Helpers for building different transport implementations for protocol,
///
pub mod async_protocol_helpers {
    use std::future::Future;
    use tokio::io::{AsyncRead, AsyncWrite};

    /// Strongly typed, Monad for a function that creates a writable stream
    ///
    pub fn write_stream<Writer, F>(
        custom_attr: &'static str,
        writer_impl: impl FnOnce(&'static str) -> F,
    ) -> impl FnOnce() -> F
    where
        Writer: AsyncWrite + Unpin,
        F: Future<Output = Writer>,
    {
        stream::<Writer, F>(custom_attr, writer_impl)
    }

    /// Strongly-typed, Monad for a function that creates a readable stream,
    ///
    pub fn read_stream<Reader, F>(
        custom_attr: &'static str,
        reader_impl: impl FnOnce(&'static str) -> F,
    ) -> impl FnOnce() -> F
    where
        Reader: AsyncRead + Unpin,
        F: Future<Output = Reader>,
    {
        stream::<Reader, F>(custom_attr, reader_impl)
    }

    /// Monad for a function that creates a stream,
    ///
    pub fn stream<IO, F>(
        custom_attr: &'static str,
        stream_impl: impl FnOnce(&'static str) -> F,
    ) -> impl FnOnce() -> F
    where
        F: Future<Output = IO>,
    {
        move || stream_impl(custom_attr)
    }
}
