use std::{ops::Deref, sync::Arc};

use specs::Write;

use crate::{editor::State, prelude::*, engine::Profiler};

/// Extension trait for Host, that provides functions for opening a GUI editor,
///
pub trait Editor {
    /// Opens this host app with the runtime editor extension,
    ///
    fn open_runtime_editor<P>(self)
    where
        P: Project;

    /// Opens this host app with an extension,
    ///
    fn open<P, E>(self, width: f64, height: f64, extension: E)
    where
        P: Project,
        E: Extension + 'static;

    /// Build appendix to look up descriptions for entities,
    ///
    fn build_appendix(&mut self);
}

impl Editor for Host {
    fn open_runtime_editor<P>(mut self)
    where
        P: Project,
    {
        self.build_appendix();
        let appendix = self.world().read_resource::<Appendix>().deref().clone();
        self.open::<P, _>(1920.0, 1080.0, WorkspaceEditor::from(appendix))
    }

    fn open<P, E>(mut self, width: f64, height: f64, extension: E)
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
            width,
            height,
            HostEditor::default(),
            extension,
            world,
            Some(builder),
        );
    }

    fn build_appendix(&mut self) {
        // Build runtime appendix
        self.world_mut().exec(
            |(entities, engines, events, thunks, graphs, profilers, mut appendix): (
                Entities,
                ReadStorage<Engine>,
                ReadStorage<Event>,
                ReadStorage<Thunk>,
                ReadStorage<AttributeGraph>,
                ReadStorage<Profiler>,
                Write<Appendix>,
            )| {
                for (entity, engine, event, thunk, graph) in
                    (&entities, engines.maybe(), events.maybe(), thunks.maybe(), graphs.maybe()).join()
                {
                    match (event, thunk, graph, engine) {
                        (None, Some(thunk), Some(graph), None) => {
                            appendix.insert_general(entity, thunk);
                            appendix.insert_state(
                                entity,
                                State {
                                    graph: graph.clone(),
                                },
                            );
                        }
                        (Some(event), None, _, None) => {
                            appendix.insert_general(entity, event);
                        }
                        (None, None, None, Some(engine)) => {
                            appendix.insert_general(entity, engine)
                        }
                        _ => {}
                    }
                }

                for (entity, _) in (&entities, &profilers).join() {
                    appendix.insert_general(entity, General { name: "profiler".to_string() })
                }
            },
        );
    }
}
