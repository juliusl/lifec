use std::{collections::HashMap, ops::Deref, sync::Arc};

use specs::Write;

use crate::{
    debugger::Debugger,
    editor::State,
    engine::{Performance, Profiler},
    prelude::*,
};

/// Extension trait for Host, that provides functions for opening a GUI editor,
///
pub trait Editor {
    /// Opens this host app with the runtime editor extension,
    ///
    fn open_runtime_editor<P>(self, enable_debugger: bool)
    where
        P: Project;

    /// Opens this host app with an extension,
    ///
    fn open<P, E>(self, extension: E)
    where
        P: Project,
        E: Extension + 'static;

    /// Opens a project that implements App,
    ///
    fn open_app<A, E>(self, app: A, extension: E)
    where
        A: Project + App + for<'a> System<'a>,
        E: Extension + 'static;

    /// Build appendix to look up descriptions for entities,
    ///
    fn build_appendix(&mut self);
}

impl Editor for Host {
    fn open_runtime_editor<P>(mut self, enable_debugger: bool)
    where
        P: Project,
    {
        self.prepare::<P>();
        self.build_appendix();
        let appendix = self.world().read_resource::<Appendix>().deref().clone();

        self.world_mut().insert(None::<Debugger>);
        self.world_mut().insert(None::<HashMap<Entity, NodeStatus>>);
        self.world_mut().insert(None::<Vec<Performance>>);

        if enable_debugger {
            self.enable_listener::<Debugger>();
        }

        let mut editor = WorkspaceEditor::from(appendix);

        editor.add_workspace_config(self.world());

        self.open::<P, _>(editor);
    }

    fn open<P, E>(mut self, extension: E)
    where
        P: Project,
        E: Extension + 'static,
    {
        // This is to initialize resources, but the actual dispatcher will be from open,
        //
        self.prepare::<P>();
        let builder = self.new_dispatcher_builder::<P>();

        // Consume Appendix and convert to read-only Arc
        if let Some(appendix) = self.world_mut().remove::<Appendix>() {
            self.world_mut().insert(Arc::new(appendix));
        }

        // Consume the compiled world
        let world = self.world.take();

        // Open the window
        atlier::prelude::open_window(
            HostEditor::name(),
            HostEditor::default(),
            extension,
            world,
            Some(builder),
        );
    }

    fn open_app<A, E>(mut self, app: A, extension: E)
    where
        A: Project + App + for<'a> System<'a>,
        E: Extension + 'static 
    {
        // This is to initialize resources, but the actual dispatcher will be from open,
        //
        self.prepare::<A>();
        let builder = self.new_dispatcher_builder::<A>();

        // Consume Appendix and convert to read-only Arc
        if let Some(appendix) = self.world_mut().remove::<Appendix>() {
            self.world_mut().insert(Arc::new(appendix));
        }

        // Consume the compiled world
        let world = self.world.take();

        // Open the window
        atlier::prelude::open_window(
            A::name(),
            app,
            extension,
            world,
            Some(builder),
        );
    }

    fn build_appendix(&mut self) {
        // Build runtime appendix
        self.world_mut().exec(
            |(entities, engines, events, thunks, graphs, profilers, blocks, mut appendix): (
                Entities,
                ReadStorage<Engine>,
                ReadStorage<Event>,
                ReadStorage<Thunk>,
                ReadStorage<AttributeGraph>,
                ReadStorage<Profiler>,
                ReadStorage<Block>,
                Write<Appendix>,
            )| {
                for (entity, block, engine, event, thunk, graph) in (
                    &entities,
                    blocks.maybe(),
                    engines.maybe(),
                    events.maybe(),
                    thunks.maybe(),
                    graphs.maybe(),
                )
                    .join()
                {
                    match (block, event, thunk, graph, engine) {
                        (Some(block), None, Some(thunk), Some(graph), None) => {
                            appendix.insert_general(entity, thunk);
                            appendix.insert_state(
                                entity,
                                State {
                                    control_symbol: block.symbol().to_string(),
                                    graph: Some(graph.clone()),
                                },
                            );
                        }
                        (block, Some(event), None, _, None) => {
                            appendix.insert_general(entity, event);
                            if let Some(block) = block {
                                appendix.insert_state(
                                    entity,
                                    State {
                                        control_symbol: block.symbol().to_string(),
                                        graph: None,
                                    },
                                );
                            }
                        }
                        (_, None, None, None, Some(engine)) => {
                            appendix.insert_general(entity, engine)
                        }
                        _ => {}
                    }
                }

                for (entity, _) in (&entities, &profilers).join() {
                    appendix.insert_general(
                        entity,
                        General {
                            name: "profiler".to_string(),
                            expression: format!("profiler-{}", entity.id()),
                        },
                    )
                }
            },
        );
    }
}
