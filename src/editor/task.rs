use specs::storage::BTreeStorage;
use specs::{Entities, WriteStorage, ReadStorage, Join, Component};

use super::{ProgressStatusBar, StartButton};
use crate::engine::Connection;
use crate::prelude::*;

/// This component drives visibility actions for running engines
/// within the runtime
/// 
#[derive(Default, Component, Clone)]
#[storage(BTreeStorage)]
pub struct Task(
    /// Start button to start a task sequence
    Option<StartButton>,
    /// Progress bar w/ status updates
    Option<ProgressStatusBar>,
    /// Sequence of events that will be executed
    Option<Sequence>,
    /// Connection to the next task
    Option<Connection>, 
    /// Disables the task, removing the start button
    bool
);

impl Task {
    /// Disables the start button on the task,
    /// 
    pub fn disable(&mut self) {
        self.4 = true;
    }
}

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
        if let Task(Some(start_button), .., connection, false) = self {
            if connection.is_some() {
                ui.text("-->");
                ui.same_line();
                if ui.is_item_hovered() {
                    ui.tooltip_text("This task has a connection component, which means it is the start of an engine.");
                }
            }
            start_button.on_ui(app_world, ui);
        }

        if let Task(_, Some(progress_status_bar), ..) = self {
            if progress_status_bar.0 > 0.0 {
                ui.same_line();
            }
            progress_status_bar.on_ui(app_world, ui);
        }

        if let Task(.., Some(sequence), _) = self {
            ui.text(format!("{:?}", sequence));
        }
    }

    fn on_run(&'_ mut self, app_world: &World) {
        if let Task(_, Some(progess_status_bar), ..) = self {
            progess_status_bar.on_run(app_world);
        }
    }
}

/// This system drives the task view, 
/// 
struct TaskSystem;

impl<'a> System<'a> for TaskSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Task>,
        ReadStorage<'a, StartButton>,
        ReadStorage<'a, ProgressStatusBar>,
        ReadStorage<'a, Sequence>,
        ReadStorage<'a, Connection>,
        ReadStorage<'a, Archive>,
    );

    fn run(&mut self, (entities, mut tasks, start_events, progress, sequences, connections, archives): Self::SystemData) {
        for (_, task, start_event, progress, sequence, connection) in (
            &entities,
            &mut tasks,
            start_events.maybe(),
            progress.maybe(),
            sequences.maybe(),
            connections.maybe(),
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
            if let Some(connection) = connection {
                task.3 = Some(connection.clone());
            }
        }

        for archive in archives.join() {
            if let Some(archived) = archive.archived() {
                if let Some(task) = tasks.get_mut(archived) {
                    task.disable();
                }
            }
        }
    }
}
