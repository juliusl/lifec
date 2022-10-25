use crate::prelude::*;
use specs::{DispatcherBuilder, System, World, Write, shred::DynamicSystemData};

type EnableListener = fn(&World, &mut DispatcherBuilder);

/// Wrapper struct to a function that install an event handler system,
///
pub struct ListenerSetup(pub EnableListener);

impl ListenerSetup {
    /// Creates a new handler setup struct,
    ///
    pub fn new<L>() -> Self
    where
        L: Listener,
    {
        Self(|world, builder| {
            builder.add(EventHandler::new(L::create(world)), "", &[]);
        })
    }
}

/// Wrapper struct over an event listener that implements a system to handle events,
///
struct EventHandler<L: Listener> {
    listener: Option<L>,
}

impl<L> EventHandler<L>
where
    L: Listener,
{
    /// Returns a new event handler,
    ///
    pub fn new(listener: L) -> Self {
        Self { listener: Some(listener) }
    }
}

impl<'a, L: Listener> System<'a> for EventHandler<L> {
    type SystemData = (
        PluginListener<'a>,
        Write<'a, tokio::sync::broadcast::Receiver<Entity>, EventRuntime>,
        Write<'a, tokio::sync::mpsc::Receiver<ErrorContext>, EventRuntime>,
        Write<'a, L>,
    );

    fn setup(&mut self, world: &mut World) {
        <Self::SystemData as DynamicSystemData>::setup(&self.accessor(), world);
    
        if let Some(listener) = self.listener.take() {
            world.insert(listener);
        }
    }

    fn run(&mut self, (mut plugin_messages,  mut completed_plugins, mut errors, mut listener): Self::SystemData) {
        if let Some(operation) = plugin_messages.try_next_operation() {
            listener.on_operation(&operation);
        }

        if let Some(runmd) = plugin_messages.try_next_runmd_file() {
            listener.on_runmd(&runmd);
        }

        if let Some(start) = plugin_messages.try_next_start_command() {
            listener.on_start_command(&start);
        }

        if let Some(status_update) = plugin_messages.try_next_status_update() {
            listener.on_status_update(&status_update);
        }

        if let Some(entity) = completed_plugins.try_recv().ok() {
            listener.on_completed_event(&entity);
        }

        if let Some(error) = errors.try_recv().ok() {
            listener.on_error_context(&error);
        }
    }
}
