use super::{ProgressStatusBar, StartButton, Task};
use crate::plugins::*;
use specs::Builder;
use specs::Entities;
use specs::Entity;
use specs::Join;
use specs::ReadStorage;
use specs::World;
use specs::WorldExt;
use specs::WriteStorage;
use crate::System;

#[derive(Clone, Default)]
pub struct Call;

impl Engine for Call {
    fn event_name() -> &'static str {
        "call"
    }

    fn init_event(entity: specs::EntityBuilder, event: Event) -> specs::EntityBuilder {
        entity
            .with(Task::default())
            .with(StartButton::default())
            .with(ProgressStatusBar::default())
            .with(event)
    }

    fn create_event(entity: Entity, world: &World) {
        let mut tasks = world.write_component::<Task>();
        let mut start_buttons = world.write_component::<StartButton>();
        let mut progress_status_bars = world.write_component::<ProgressStatusBar>();

        match tasks.insert(entity, Task::default()) {
            Ok(_) => {}
            Err(_) => {}
        }

        match start_buttons.insert(entity, StartButton::default()) {
            Ok(_) => {}
            Err(_) => {}
        }

        match progress_status_bars.insert(entity, ProgressStatusBar::default()) {
            Ok(_) => {}
            Err(_) => {}
        }
    }
}

impl<'a> System<'a> for Call {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, ThunkContext>,
        WriteStorage<'a, StartButton>,
        WriteStorage<'a, Event>,
        WriteStorage<'a, CancelThunk>,
    );

    fn run(&mut self, (entities, contexts, mut start_buttons, mut events, mut _cancel_thunk): Self::SystemData) {
        for (entity, context, start_button, event) in
            (&entities, &contexts, &mut start_buttons, &mut events).join()
        {
            // Handle starting the event
            if let Some(true) = start_button.0 {
                if let Some(cancel) = _cancel_thunk.remove(entity) {
                    cancel.0.send(()).ok();
                } else {
                    event.fire(context.clone());
                }

                start_button.0 = Some(false);
            }

            // Handle setting the current status
            if let Some(_) = start_button.0 {
                if event.is_running() {
                    start_button.1 = "Running".to_string();
                } else {
                    start_button.1 = context
                        .as_ref()
                        .find_text("elapsed")
                        .and_then(|e| Some(format!("Completed, elapsed: {}", e)))
                        .unwrap_or("Completed".to_string());
                }
            }

            // Sets the label for this button
            start_button.2 = { 
                if event.is_running() {
                    format!("cancel {}", event.to_string())
                } else {
                    event.to_string()
                }
            };

            // Sets the owning entity
            start_button.3 = Some(entity);
        }
    }
}
