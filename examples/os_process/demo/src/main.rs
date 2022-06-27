use std::time::Duration;

use lifec::plugins::{Engine, Event, EventRuntime, Plugin, ThunkContext, Process};
use lifec::{editor::*, AttributeGraph, Runtime};
use specs::storage::DenseVecStorage;
use specs::{
    Component, DispatcherBuilder, Entities, Join, ReadStorage, RunNow, System, World, WriteStorage, Entity,
};
use tokio::task::JoinHandle;
use tokio::time::Instant;

fn main() {
    if let Some(file) = AttributeGraph::load_from_file("test_demo.runmd") {
        open(
            "demo",
            RuntimeEditor::new(Runtime::from(file)),
            Timer::default(),
        );
    }
}

#[derive(Default, Component, Clone)]
#[storage(DenseVecStorage)]
struct Timer(
    Option<StartEvent>, 
    Option<Progress>
);

impl Plugin<ThunkContext> for Timer {
    fn symbol() -> &'static str {
        "timer"
    }

    fn call_with_context(thunk_context: &mut ThunkContext) -> Option<JoinHandle<ThunkContext>> {
        thunk_context.clone().task(|| {
            let tc = thunk_context.clone();
            async move {
                let mut duration = 5;
                if let Some(d) = tc.as_ref().find_int("duration") {
                    tc.update_progress("duration found", 0.0).await;
                    duration = d;
                }

                let start = Instant::now();
                let duration = duration as u64;
                loop {
                    let elapsed = start.elapsed();
                    let progress =
                        elapsed.as_secs_f32() / Duration::from_secs(duration).as_secs_f32();
                    if progress < 1.0 {
                        tc.update_progress(format!("elapsed {} ms", elapsed.as_millis()), progress)
                            .await;
                    } else {
                        break;
                    }
                }

                None
            }
        })
    }
}

#[derive(Clone)]
struct Start;

impl Engine for Start {
    fn event_name() -> &'static str {
        "start"
    }
}

impl<'a> System<'a> for Start {
    type SystemData = (
        WriteStorage<'a, StartEvent>,
        ReadStorage<'a, ThunkContext>,
        WriteStorage<'a, Event>,
    );

    fn run(&mut self, (mut start_events, contexts, mut events): Self::SystemData) {
        for (start_event, context, event) in (&mut start_events, &contexts, &mut events).join() {
            if start_event.0 {
                event.fire(context.clone());
                start_event.0 = false;
            }

            if event.is_running() {
                start_event.1 = "Running";
            } else {
                start_event.1 = "Completed";
            }
        }
    }
}

#[derive(Component, Clone)]
#[storage(DenseVecStorage)]
struct StartEvent(bool, &'static str, Option<Entity>);

impl Default for StartEvent
{
    fn default() -> Self {
        Self(Default::default(), Default::default(), None)
    }
}

impl App for StartEvent
{
    fn name() -> &'static str {
        "start_event"
    }

    fn edit_ui(&mut self, ui: &imgui::Ui) {
        self.0 = ui.button(format!("start"));
    }

    fn display_ui(&self, ui: &imgui::Ui) {
        ui.same_line();
        ui.text(&self.1);
    }
}

impl Extension for StartEvent {
    fn configure_app_world(_: &mut World) {
        // todo!()
    }

    fn configure_app_systems(dispatcher: &mut DispatcherBuilder) {
        dispatcher.add(Start{}, "start_event", &[]);
    }

    fn on_ui(&'_ mut self, app_world: &World, ui: &'_ imgui::Ui<'_>) {
        if let Self(.., Some(entity)) = self {
            let mut components =  app_world.write_component::<Self>();
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

impl Extension for Timer {
    fn configure_app_world(world: &mut World) {
        EventRuntime::configure_app_world(world);
        world.register::<StartEvent>();
        world.register::<Progress>();

        let mut initial_context = ThunkContext::default();
        initial_context.as_mut().add_int_attr("duration", 5);
        world
            .create_entity()
            .with(initial_context)
            .with(Start::event::<Timer>())
            .with(StartEvent::default())
            .with(Progress::default())
            .build();
    }

    fn configure_app_systems(dispatcher: &mut DispatcherBuilder) {
        StartEvent::configure_app_systems(dispatcher);
        EventRuntime::configure_app_systems(dispatcher);
    }

    fn on_ui(&'_ mut self, app_world: &World, ui: &'_ imgui::Ui<'_>) {
        if let Timer(Some(start_event), Some(progress)) = self {
            start_event.on_ui(app_world, ui);
            progress.on_ui(app_world, ui);
        }
        ui.show_demo_window(&mut true);
    }

    fn on_window_event(&'_ mut self, _: &World, event: &'_ WindowEvent<'_>) {
        match event {
            WindowEvent::DroppedFile(file) => {
                println!("File dropped {:?}", file);
            }
            _ => {}
        }
    }

    fn on_run(&'_ mut self, app_world: &World) {
        if let Some(progress) = self.1.as_mut() {
            progress.on_run(app_world);
        }

        self.run_now(app_world);
    }
}

impl<'a> System<'a> for Timer {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, StartEvent>,
        ReadStorage<'a, Progress>,
    );

    fn run(&mut self, (entities, start_events, progress): Self::SystemData) {
        for (entity, start_event, progress) in (&entities, start_events.maybe(), progress.maybe()).join() {
            if let Some(start_event) = start_event {
                let mut start_event = start_event.clone();
                start_event.2 = Some(entity);
                self.0 = Some(start_event);
            }
            if let Some(progress) = progress {
                self.1 = Some(progress.clone());
            }
        }
    }
}
