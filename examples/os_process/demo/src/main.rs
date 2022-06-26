use std::any::Any;
use std::time::Duration;

use lifec::plugins::{Engine, Event, EventRuntime, Plugin, ThunkContext};
use lifec::{editor::*, AttributeGraph, Runtime};
use specs::storage::{self, DenseVecStorage};
use specs::{
    Component, DispatcherBuilder, Entities, Join, ReadStorage, RunNow, System, World, WriteStorage,
};
use tokio::pin;
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
struct Timer(Option<StartEvent<Timer, Timer>>, Option<Progress>);

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
                        //tc.update_status_only(format!("elapsed {:?}", elapsed)).await;
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

impl Engine<Timer> for Timer {
    fn event_name() -> &'static str {
        "start"
    }

    fn setup(_: &mut AttributeGraph) {}

    fn init(entity: specs::EntityBuilder) -> specs::EntityBuilder {
        entity
            .with(StartEvent::<Timer, Timer>::default())
            .with(Progress::default())
    }
}

#[derive(Component, Clone)]
#[storage(DenseVecStorage)]
struct StartEvent<E, P>(bool, String, fn(E, P))
where
    E: Engine<P> + Send + Sync + Any,
    P: Plugin<ThunkContext> + Component + Send + Default,
    <P as Component>::Storage: Default;

impl<E, P> Default for StartEvent<E, P>
where
    E: Engine<P> + Send + Sync + Any,
    P: Plugin<ThunkContext> + Component + Send + Default,
    <P as Component>::Storage: Default,
{
    fn default() -> Self {
        Self(Default::default(), Default::default(), |_, _| {})
    }
}

impl<E, P> Extension for StartEvent<E, P>
where
    E: Engine<P> + Send + Sync + Any,
    P: Plugin<ThunkContext> + Component + Send + Default,
    <P as Component>::Storage: Default,
{
    fn configure_app_world(world: &mut World) {
        world.register::<P>();
    }

    fn configure_app_systems(_: &mut DispatcherBuilder) {}

    fn on_ui(&'_ mut self, _: &World, ui: &'_ imgui::Ui<'_>) {
        self.edit_ui(ui);
        self.display_ui(ui);
    }

    fn on_window_event(&'_ mut self, _: &World, _: &'_ WindowEvent<'_>) {}

    fn on_run(&'_ mut self, world: &World) {
        self.run_now(world);
    }
}

impl <E, P> App for StartEvent<E, P> 
where
    E: Engine<P> + Send + Sync + Any,
    P: Plugin<ThunkContext> + Component + Send + Default,
    <P as Component>::Storage: Default
{
    fn name() -> &'static str {
        "start_event"
    }

    fn edit_ui(&mut self, ui: &imgui::Ui) {
        self.0 = ui.button(format!("{} {}", E::event_name(), P::symbol()));
    }

    fn display_ui(&self, ui: &imgui::Ui) {
        ui.same_line();
        ui.text(&self.1);
    }
}

impl<'a, E, P> System<'a> for StartEvent<E, P> 
where
    E: Engine<P> + Send + Sync + Any,
    P: Plugin<ThunkContext> + Component + Send + Default,
    <P as Component>::Storage: Default,
{
    type SystemData = (
        ReadStorage<'a, ThunkContext>,
        WriteStorage<'a, Event>,
        WriteStorage<'a, StartEvent<E, P>>,
    );

    fn run(&mut self, (thunk_contexts, mut events, mut start_events): Self::SystemData) {
        for (context, event) in (&thunk_contexts, &mut events).join() {
            if self.0 {
                event.fire(context.clone());
                self.0 = false;
            }

            if event.is_running() {
                self.1 = "Running".to_string();
            } else {
                self.1 = "Stopped".to_string();
            }

            if let Some(entity) = context.entity {
                if let Some(start_event) = start_events.get_mut(entity) {
                    start_event.1 = self.1.to_string();
                }
            }
        }
    }
}

impl Extension for Timer {
    fn configure_app_world(world: &mut World) {
        EventRuntime::configure_app_world(world);
        world.register::<StartEvent<Self, Self>>();
        world.register::<Progress>();

        let mut initial_context = ThunkContext::default();
        initial_context.as_mut().add_int_attr("duration", 5);
        world
            .create_entity()
            .with(initial_context)
            .with(Timer::event())
            .with(StartEvent::<Timer, Timer>::default())
            .with(Progress::default())
            .build();
    }

    fn configure_app_systems(dispatcher: &mut DispatcherBuilder) {
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
        if let Some(start_event) = self.0.as_mut() {
            start_event.on_run(app_world);
        }

        if let Some(progress) = self.1.as_mut() {
            progress.on_run(app_world);
        }

        self.run_now(app_world);
    }
}

impl<'a> System<'a> for Timer {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, StartEvent<Self, Self>>,
        ReadStorage<'a, Progress>,
    );

    fn run(&mut self, (entities, start_events, progress): Self::SystemData) {
        for (_, start_event, progress) in (&entities, start_events.maybe(), progress.maybe()).join() {
            if let Some(start_event) = start_event {
                self.0 = Some(start_event.clone());
            }
            if let Some(progress) = progress {
                self.1 = Some(progress.clone());
            }
        }
    }
}
