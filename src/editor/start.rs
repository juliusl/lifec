use atlier::system::{App, Extension, WindowEvent};
use specs::storage::DenseVecStorage;
use specs::{
    Builder, Component, DispatcherBuilder, Entities, Entity, Join, ReadStorage, System, World,
    WorldExt, WriteStorage,
};

use super::ProgressStatusBar;
use crate::plugins::{Engine, Event, EventRuntime, ThunkContext};

#[derive(Clone)]
pub struct Start;

impl Engine for Start {
    fn event_name() -> &'static str {
        "start"
    }

    fn init(entity: specs::EntityBuilder) -> specs::EntityBuilder {
        entity
            .with(StartButton::default())
            .with(ProgressStatusBar::default())
    }
}

impl<'a> System<'a> for Start {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, ThunkContext>,
        WriteStorage<'a, StartButton>,
        WriteStorage<'a, Event>,
    );

    fn run(&mut self, (entities, contexts, mut start_buttons, mut events): Self::SystemData) {
        for (entity, context, start_button, event) in
            (&entities, &contexts, &mut start_buttons, &mut events).join()
        {
            // Handle starting the event
            if start_button.0 {
                event.fire(context.clone());
                start_button.0 = false;
            }

            // Handle setting the current status
            if event.is_running() {
                start_button.1 = "Running".to_string();
            } else {
                start_button.1 = context
                    .as_ref()
                    .find_text("elapsed")
                    .and_then(|e| Some(format!("Completed, elapsed: {}", e)))
                    .unwrap_or("Completed".to_string());
            }

            // Sets the label for this button
            start_button.2 = event.to_string();

            // Sets the owning entity
            start_button.3 = Some(entity);
        }
    }
}

#[derive(Component, Clone, Default)]
#[storage(DenseVecStorage)]
pub struct StartButton(bool, String, String, Option<Entity>);

impl App for StartButton {
    fn name() -> &'static str {
        "start_button"
    }

    fn edit_ui(&mut self, ui: &imgui::Ui) {
        if let Self(.., label, Some(entity)) = self {
            self.0 = ui.button(format!("{} {}", label, entity.id()));
        }
    }

    fn display_ui(&self, ui: &imgui::Ui) {
        ui.same_line();
        ui.text(&self.1);
    }
}

impl Extension for StartButton {
    fn configure_app_world(_: &mut World) {
        // todo!()
    }

    fn configure_app_systems(dispatcher: &mut DispatcherBuilder) {
        dispatcher.add(Start {}, "start_event", &[]);
        dispatcher.add(
            EventRuntime::default(),
            "start_event_runtime",
            &["start_event"],
        );
    }

    fn on_ui(&'_ mut self, app_world: &World, ui: &'_ imgui::Ui<'_>) {
        if let Self(.., Some(entity)) = self {
            let mut components = app_world.write_component::<Self>();
            if let Some(start_event) = components.get_mut(*entity) {
                start_event.edit_ui(ui);
                start_event.display_ui(ui);
            }
        }
    }

    fn on_window_event(&'_ mut self, _: &World, _: &'_ WindowEvent<'_>) {
        //todo!()
    }

    fn on_run(&'_ mut self, _: &World) {
        //todo!()
    }
}
