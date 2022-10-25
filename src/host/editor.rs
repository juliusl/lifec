use std::{collections::HashSet, sync::Arc};

use specs::Write;

use crate::prelude::*;

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
        self.open::<P, _>(1920.0, 1080.0, WorkspaceEditor::default())
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

        // Get defined workspace operations
        let set = {
            let operations = self.world().system_data::<Operations>();
            HashSet::from_iter(operations.scan_root().iter().cloned())
        };

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
            HostEditor::from(set),
            extension,
            world,
            Some(builder),
        );
    }
    
    fn build_appendix(&mut self) {
        // Build runtime appendix
        self.world_mut().exec(
            |(entities, events, thunks, mut appendix): (
                Entities,
                ReadStorage<Event>,
                ReadStorage<Thunk>,
                Write<Appendix>,
            )| {
                for (entity, event, thunk) in (&entities, events.maybe(), thunks.maybe()).join() {
                    match (event, thunk) {
                        (None, Some(thunk)) => {
                            appendix.insert_general(entity, thunk);
                        }
                        (Some(event), None) => {
                            appendix.insert_general(entity, event);
                        }
                        _ => {}
                    }
                }
            },
        );
    }
}
