use super::{ProgressStatusBar, StartButton};
use crate::plugins::*;
use atlier::system::Extension;
use specs::storage::BTreeStorage;

#[derive(Default, Component, Clone)]
#[storage(BTreeStorage)]
pub struct Task(Option<StartButton>, Option<ProgressStatusBar>, Option<Sequence>);

impl Extension for Task {
    fn configure_app_world(world: &mut World) {
        EventRuntime::configure_app_world(world);
        StartButton::configure_app_world(world);
        world.register::<ProgressStatusBar>();
        world.register::<Task>();
    }

    fn configure_app_systems(dispatcher: &mut DispatcherBuilder) {
        StartButton::configure_app_systems(dispatcher);
        dispatcher.add(TaskSystem {}, "task_system", &[]);
    }

    fn on_ui(&'_ mut self, app_world: &World, ui: &'_ imgui::Ui<'_>) {
        if let Task(Some(start_button), ..) = self {
            start_button.on_ui(app_world, ui);
        }

        if let Task(_, Some(progress_status_bar), ..) = self {
            if progress_status_bar.0 > 0.0 {
                ui.same_line();
            }
            progress_status_bar.on_ui(app_world, ui);
        }

        if let Task(.., Some(sequence)) = self {
            ui.text(format!("{}", sequence));
        }
    }

    fn on_run(&'_ mut self, app_world: &World) {
        if let Task(_, Some(progess_status_bar), ..) = self {
            progess_status_bar.on_run(app_world);
        }
    }
}

struct TaskSystem;

impl<'a> System<'a> for TaskSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Task>,
        ReadStorage<'a, StartButton>,
        ReadStorage<'a, ProgressStatusBar>,
        ReadStorage<'a, Sequence>,
    );

    fn run(&mut self, (entities, mut tasks, start_events, progress, sequences): Self::SystemData) {
        for (_, task, start_event, progress, sequence) in (
            &entities,
            &mut tasks,
            start_events.maybe(),
            progress.maybe(),
            sequences.maybe(),
        )
            .join()
        {
            if let Some(start_event) = start_event {
                task.0 = Some(start_event.clone());
            }
            if let Some(progress) = progress {
                task.1 = Some(progress.clone());
            }
            if let Some(sequence) = sequence {
                task.2 = Some(sequence.clone());
            }
        }
    }
}