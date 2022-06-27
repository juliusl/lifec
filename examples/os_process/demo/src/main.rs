use std::time::Duration;

use lifec::plugins::{Engine, EventRuntime, Plugin, ThunkContext};
use lifec::{editor::*, AttributeGraph, Runtime};
use specs::storage::DenseVecStorage;
use specs::{
    Component, DispatcherBuilder, Entities, Join, ReadStorage, System, World, WriteStorage,
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
    Option<StartButton>, 
    Option<ProgressStatusBar>
);

impl Plugin<ThunkContext> for Timer {
    fn symbol() -> &'static str {
        "timer"
    }

    fn call_with_context(thunk_context: &mut ThunkContext) -> Option<JoinHandle<ThunkContext>> {
        thunk_context.clone().task(|| {
            let mut tc = thunk_context.clone();
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
                        tc.as_mut().add_text_attr("elapsed", format!("{:?}", elapsed));
                        break;
                    }
                }

                Some(tc)
            }
        })
    }
}

impl Extension for Timer {
    fn configure_app_world(world: &mut World) {
        EventRuntime::configure_app_world(world);
        world.register::<StartButton>();
        world.register::<ProgressStatusBar>();
        world.register::<Timer>();

        let mut initial_context = ThunkContext::default();
        initial_context.as_mut().add_int_attr("duration", 5);
        Start::init(world
            .create_entity()
            .with(initial_context)
            .with(Start::event::<Timer>())
            .with(Timer::default())
        ).build();
    }

    fn configure_app_systems(dispatcher: &mut DispatcherBuilder) {
        StartButton::configure_app_systems(dispatcher);
        dispatcher.add(TimerSystem{}, "timer_system", &[])
    }

    fn on_ui(&'_ mut self, app_world: &World, ui: &'_ imgui::Ui<'_>) {
        let mut timers = app_world.write_component::<Timer>();

        for timer in timers.as_mut_slice() {
            if let Timer(Some(start_button), Some(progress_status_bar)) = timer {
                start_button.on_ui(app_world, ui);
                progress_status_bar.on_ui(app_world, ui);
            }
        }
    }

    fn on_window_event(&'_ mut self, _: &World, _: &'_ WindowEvent<'_>) {
    }

    fn on_run(&'_ mut self, _: &World) {
        //
    }
}

struct TimerSystem;

impl<'a> System<'a> for TimerSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Timer>,
        ReadStorage<'a, StartButton>,
        ReadStorage<'a, ProgressStatusBar>,
    );

    fn run(&mut self, (entities, mut timers, start_events, progress): Self::SystemData) {
        for (_, timer, start_event, progress) in (&entities, &mut timers, start_events.maybe(), progress.maybe()).join() {
            if let Some(start_event) = start_event {
                timer.0 = Some(start_event.clone());
            }
            if let Some(progress) = progress {
                timer.1 = Some(progress.clone());
            }
        }
    }
}
