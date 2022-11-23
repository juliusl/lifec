use crate::appendix::Appendix;

cfg_editor! {
    use std::{collections::HashMap, ops::Deref, sync::Arc};

    use crate::{
        debugger::Debugger,
        engine::Performance,
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
    }
}
