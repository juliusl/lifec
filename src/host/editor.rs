use atlier::system::{App, Extension};
use specs::{Entities, Join, ReadStorage, System, WorldExt, WriteStorage};

use crate::{
    editor::{ProgressStatusBar, StartButton, Task},
    engine::Connection,
    prelude::CancelThunk,
    AttributeIndex, Event, Host, RuntimeEditor, ThunkContext,
};

/// Extension trait for Host, that provides functions for opening a GUI editor,
///
pub trait Editor {
    /// Opens this host app with the runtime editor extension,
    ///
    fn open_runtime_editor(self);

    /// Opens this host app with an extension,
    ///
    fn open<E>(self, width: f64, height: f64, extension: E)
    where
        E: Extension + 'static;
}

impl Editor for Host {
    fn open_runtime_editor(mut self) {
        // Register some common components for viewing event state
        self.world_mut().register::<Task>();
        self.world_mut().register::<ProgressStatusBar>();
        self.world_mut().register::<StartButton>();
        self.world_mut().register::<Connection>();

        // Setup task list view --
        self.world_mut().exec(
            |(
                entities,
                events,
                mut tasks,
                mut progress_bars,
                mut start_buttons,
                mut connections,
                mut contexts,
            ): (
                Entities,
                ReadStorage<Event>,
                WriteStorage<Task>,
                WriteStorage<ProgressStatusBar>,
                WriteStorage<StartButton>,
                WriteStorage<Connection>,
                WriteStorage<ThunkContext>,
            )| {
                for (entity, _) in (&entities, &events).join() {
                    tasks
                        .insert(entity, Task::default())
                        .expect("should be able to insert task");
                    progress_bars
                        .insert(entity, ProgressStatusBar::default())
                        .expect("should be able to insert progress status bar");
                    start_buttons
                        .insert(entity, StartButton::default())
                        .expect("should be able to insert start button");
              
                    contexts
                        .insert(entity, ThunkContext::default())
                        .expect("should be able to insert a thunk context");
                }
            },
        );
        self.world_mut().maintain();
        
        self.open(1920.0, 1080.0, RuntimeEditor::default())
    }

    fn open<E>(mut self, width: f64, height: f64, extension: E)
    where
        E: Extension + 'static,
    {
        // Consume the compiled world
        let world = self.world.take();

        // Open the window
        atlier::prelude::open_window(Self::name(), width, height, self, extension, world);
    }
}

impl<'a> System<'a> for Host {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, ThunkContext>,
        WriteStorage<'a, StartButton>,
        WriteStorage<'a, Event>,
        WriteStorage<'a, CancelThunk>,
    );

    fn run(
        &mut self,
        (entities, contexts, mut start_buttons, mut events, mut _cancel_thunk): Self::SystemData,
    ) {
        for (entity, context, start_button, event) in
            (&entities, &contexts, &mut start_buttons, &mut events).join()
        {
            // Handle starting the event
            // if let Some(true) = start_button.0 {
            //     if let Some(cancel) = _cancel_thunk.remove(entity) {
            //         cancel.0.send(()).ok();
            //     } else {
            //         event.fire(context.clone());
            //     }

            //     start_button.0 = Some(false);
            // }

            // // Handle setting the current status
            // if let Some(_) = start_button.0 {
            //     if event.is_running() {
            //         start_button.1 = "Running".to_string();
            //     } else {
            //         start_button.1 = context
            //             .state()
            //             .find_text("elapsed")
            //             .and_then(|e| Some(format!("Completed, elapsed: {}", e)))
            //             .unwrap_or("Completed".to_string());
            //     }
            // }

            // // Sets the label for this button
            // start_button.2 = {
            //     if event.is_running() {
            //         format!("cancel {}", event.to_string())
            //     } else {
            //         event.to_string()
            //     }
            // };

            // Sets the owning entity
            start_button.3 = Some(entity);
        }
    }
}

impl App for Host {
    fn name() -> &'static str {
        "Lifec - Runtime Editor"
    }

    fn edit_ui(&mut self, ui: &imgui::Ui) {
        // No-op
    }

    fn display_ui(&self, ui: &imgui::Ui) {
        // No-op
    }
}
