use std::time::Duration;

use imgui::Window;
use lifec::plugins::{Event, EventRuntime, Plugin, Progress, ThunkContext, Engine};
use lifec::{editor::*, AttributeGraph, Runtime};
use specs::storage::DenseVecStorage;
use specs::{
    Component, DispatcherBuilder, Entities, Join, ReadStorage, RunNow, System, World, WriteStorage,
};
use tokio::task::JoinHandle;
use tokio::time::{sleep, Instant};

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
        thunk_context.clone().task(|entity| {
            let tc = thunk_context.clone();
            async move {
                tc.update_progress(entity, "timer started", 0.01).await;
                
                let mut duration = 10;
                if let Some(d) = tc.as_ref().find_int("duration") {
                    tc.update_progress(entity, "duration found", 0.01).await;
                   duration = d;
                }

                let start = Instant::now();
                for i in 1..duration + 1 {
                    sleep(Duration::from_secs(1)).await;
                    let progress = i as f32/ (duration as f32);
                    tc.update_progress(entity, format!("elapsed {:?} {} %", start.elapsed(), progress*100.0), progress).await;
                }
                
                tc.update_progress(entity, "timer completed", 1.0).await;
                None
            }
        })
    }
}

impl Engine<Timer> for Timer {
    fn event_name() -> &'static str {
        "start"
    }

    fn event() -> Event {
        Event::from_plugin::<Self>(Self::event_name())
    }

    fn setup(_: &mut AttributeGraph) {
        
    }
}

impl Extension for Timer {
    fn configure_app_world(world: &mut World) {
        EventRuntime::configure_app_world(world);

        let mut initial_context = ThunkContext::default();
        initial_context.as_mut().add_int_attr("duration", 5);
        world.create_entity()
            .with(initial_context)
            .with(Timer::event())
            .build();
    }

    fn configure_app_systems(dispatcher: &mut DispatcherBuilder) {
        EventRuntime::configure_app_systems(dispatcher);
    }

    fn on_ui(&'_ mut self, _: &World, ui: &'_ imgui::Ui<'_>) {
        Window::new("Timer").size([800.0, 600.0], imgui::Condition::Appearing).build(ui, ||{
            ui.text(&self.1);
            if ui.button("fire") {
                self.0 = true;
            }
    
            if let Some(progress) = &self.2 {
                progress.display_ui(ui);
            }
        });
    }

    fn on_window_event(&'_ mut self, _: &World, event: &'_ WindowEvent<'_>) {
        match  event {
            WindowEvent::DroppedFile(file) => {
                println!("File dropped {:?}", file);
            },
            _ => {}
        }
    }

    fn on_run(&'_ mut self, app_world: &World) {
        self.run_now(app_world);
        EventRuntime{}.on_run(app_world);
    }
}

impl<'a> System<'a> for Timer {
    type SystemData = (Entities<'a>, ReadStorage<'a, ThunkContext>, WriteStorage<'a, Event>, ReadStorage<'a, Progress>);

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
