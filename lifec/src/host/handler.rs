use crate::{engine::NodeCommand, prelude::*};
use specs::{shred::DynamicSystemData, DispatcherBuilder, LazyUpdate, System, World, Write};

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
            builder.add(
                EventHandler::new(L::create(world)),
                "listener",
                &["event_runtime", "cleanup"],
            );
        })
    }
}

/// Wrapper struct over an event listener that implements a system to handle events,
///
#[derive(Default)]
pub struct EventHandler<L: Listener> {
    listener: Option<L>,
}

impl<L> EventHandler<L>
where
    L: Listener,
{
    /// Returns a new event handler,
    ///
    pub fn new(listener: L) -> Self {
        Self {
            listener: Some(listener),
        }
    }
}

impl<'a, L: Listener> System<'a> for EventHandler<L> {
    type SystemData = (
        Read<'a, LazyUpdate>,
        Entities<'a>,
        PluginListener<'a>,
        Write<'a, tokio::sync::broadcast::Receiver<Entity>, EventRuntime>,
        Write<'a, tokio::sync::mpsc::Receiver<ErrorContext>, EventRuntime>,
        Write<'a, Option<L>>,
    );

    fn setup(&mut self, world: &mut World) {
        <Self::SystemData as DynamicSystemData>::setup(&self.accessor(), world);

        world.insert(self.listener.take());
    }

    fn run(
        &mut self,
        (
            lazy_updates,
            entities,
            mut plugin_messages,
            mut completed_plugins,
            mut errors,
            mut listener,
        ): Self::SystemData,
    ) {
        if let Some(listener) = listener.as_mut() {
            if let Some(operation) = plugin_messages.try_next_operation() {
                listener.on_operation(operation);
            }

            if let Some(status_update) = plugin_messages.try_next_status_update() {
                listener.on_status_update(&status_update);
            }

            if let Some(completion) = plugin_messages.try_next_completion() {
                listener.on_completion(completion);
            }

            if let Some(entity) = completed_plugins.try_recv().ok() {
                listener.on_completed_event(&entity);
            }

            if let Some(error) = errors.try_recv().ok() {
                listener.on_error_context(&error);
            }

            if let Some(guest) = plugin_messages.try_next_guest() {
                lazy_updates.insert(guest.owner, guest);
            }

            if let Some((command, yielding)) = plugin_messages.try_next_node_command() {
                event!(Level::DEBUG, "Received command, {command}");
                match command {
                    NodeCommand::Activate(entity)
                    | NodeCommand::Reset(entity)
                    | NodeCommand::Pause(entity)
                    | NodeCommand::Resume(entity)
                    | NodeCommand::Cancel(entity)
                    | NodeCommand::Spawn(entity)
                    | NodeCommand::Custom(_, entity) => {
                        lazy_updates.insert(entity, command.clone());
                        if let Some(yielding) = yielding {
                            lazy_updates.insert(entity, yielding);
                        }
                    }
                    NodeCommand::Update(graph) => {
                        let entity = entities.entity(graph.entity_id());
                        lazy_updates.insert(entity, NodeCommand::Update(graph.clone()));
                    }
                    NodeCommand::Swap { owner, from, to } => {
                        lazy_updates.insert(owner, NodeCommand::Swap { owner, from, to });
                    }
                }
            }
        }
    }
}
