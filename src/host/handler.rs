use crate::{engine::{State}, prelude::*};
use specs::{shred::DynamicSystemData, DispatcherBuilder, System, World, Write};

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
        Entities<'a>,
        State<'a>,
        Write<'a, Option<L>>,
    );

    fn setup(&mut self, world: &mut World) {
        <Self::SystemData as DynamicSystemData>::setup(&self.accessor(), world);

        world.insert(self.listener.take());
    }

    fn run(
        &mut self,
        (entities, mut state, mut listener): Self::SystemData,
    ) {
        if let Some(listener) = listener.as_mut() {
            if let Some(operation) = state.plugin_listeners().try_next_operation() {
                listener.on_operation(operation);
            }

            if let Some(runmd) = state.plugin_listeners().try_next_runmd_file() {
                listener.on_runmd(&runmd);
            }

            if let Some(start) = state.plugin_listeners().try_next_start_command() {
                listener.on_start_command(&start);
            }

            if let Some(status_update) = state.plugin_listeners().try_next_status_update() {
                listener.on_status_update(&status_update);
            }

            if let Some(entity) = state.plugin_listeners().completed_entities.try_recv().ok() {
                listener.on_completed_event(&entity);
            }

            if let Some(error) =state.plugin_listeners().errors.try_recv().ok() {
                listener.on_error_context(&error);
            }

            if let Some(guest) = state.plugin_listeners().try_next_guest() {
                state.guests
                    .insert(guest.owner, guest)
                    .expect("should be able to insert guest");
            }

            if let Some((command, yielding)) = state.plugin_listeners().try_next_node_command() {
                match command {
                    NodeCommand::Activate(entity)
                    | NodeCommand::Reset(entity)
                    | NodeCommand::Pause(entity)
                    | NodeCommand::Resume(entity)
                    | NodeCommand::Cancel(entity)
                    | NodeCommand::Spawn(entity)
                    | NodeCommand::Custom(_, entity) => {
                        state.commands
                            .insert(entity, command)
                            .expect("should be able to insert");
                        if let Some(yielding) = yielding {
                            state.yielding
                                .insert(entity, yielding)
                                .expect("should be able to insert");
                        }
                    }
                    NodeCommand::Update(graph) => {
                        let entity = entities.entity(graph.entity_id());
                        state.commands
                            .insert(entity, NodeCommand::Update(graph.clone()))
                            .expect("should be able to insert");
                    }
                }
            }
        }
    }
}
