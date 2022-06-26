use std::time::Duration;

use lifec::plugins::{Engine, Event, EventRuntime, Plugin,  ThunkContext};
use lifec::{editor::*, AttributeGraph, Runtime};
use specs::storage::DenseVecStorage;
use specs::{
    Component, DispatcherBuilder, Entities, Join, ReadStorage, RunNow, System, World, WriteStorage,
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

#[derive(Default, Component)]
#[storage(DenseVecStorage)]
struct Timer(bool, String, Option<Progress>);

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
                    let progress =  elapsed.as_secs_f32() / Duration::from_secs(duration).as_secs_f32();
                    if progress < 1.0 {
                        //tc.update_status_only(format!("elapsed {:?}", elapsed)).await;
                        tc.update_progress(
                            format!("elapsed {} ms", elapsed.as_millis()),
                            progress,
                        )
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

impl Engine<Timer> for Timer {
    fn event_name() -> &'static str {
        "start"
    }

    fn setup(_: &mut AttributeGraph) {}
}

impl Extension for Timer {
    fn configure_app_world(world: &mut World) {
        EventRuntime::configure_app_world(world);
        world.register::<Progress>();

        let mut initial_context = ThunkContext::default();
        initial_context.as_mut().add_int_attr("duration", 5);
        world
            .create_entity()
            .with(initial_context)
            .with(Timer::event())
            .with(Progress::default())
            .build();
    }

    fn configure_app_systems(dispatcher: &mut DispatcherBuilder) {
        EventRuntime::configure_app_systems(dispatcher);
    }

    fn on_ui(&'_ mut self, _: &World, ui: &'_ imgui::Ui<'_>) {
        self.0 = ui.button(format!("{} {}", Timer::event_name(), Timer::symbol()));
        ui.same_line();
        ui.text(&self.1);

        if let Some(progress) = &self.2 {
            progress.display_ui(ui);
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
        if let Some(progress) = self.2.as_mut() {
            progress.on_run(app_world);
        }

        self.run_now(app_world);
    }
}

impl<'a> System<'a> for Timer {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, ThunkContext>,
        WriteStorage<'a, Event>,
        ReadStorage<'a, Progress>,
    );

    fn run(&mut self, (entities, thunk_contexts, mut events, progress): Self::SystemData) {
        for (_, thunk_context, event) in (&entities, &thunk_contexts, &mut events).join() {
            if self.0 {
                event.fire(thunk_context.clone());
                self.0 = false;
            }

            if event.is_running() {
                self.1 = "Running".to_string();
            } else {
                self.1 = "Stopped".to_string();
            }
        }

        for (_, progress) in (&entities, progress.maybe()).join() {
            if let Some(progress) = progress {
                self.2 = Some(progress.clone());
            }
        }
    }
}
