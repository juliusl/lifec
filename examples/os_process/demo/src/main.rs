use std::time::Duration;

use lifec::plugins::{Event, EventRuntime, ProgressBar, Plugin, Progress, ThunkContext, Engine};
use lifec::{editor::*, AttributeGraph, Runtime};
use specs::storage::DenseVecStorage;
use specs::{
    Component, DispatcherBuilder, Entities, Join, ReadStorage, RunNow, System, World, WriteStorage,
};
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

    fn call_with_context(_: &mut ThunkContext) {
        println!("timer finished");
    }
}

impl Engine for Timer {
    fn event_name() -> &'static str {
        "start_timer"
    }

    fn event() -> Event {
        Event::from_plugin_with::<Self>(Self::event_name(), 
              |entity, thunk, initial_context, _status_sender, handle| {
                  let thunk = thunk.clone();
                  let initial_context = initial_context.clone();
                  handle.spawn(async move {
                      let progress_bar = ProgressBar(_status_sender);
                      progress_bar.update_status(entity, "timer started", 0.01).await;
                      if let Some(Value::Int(duration)) = initial_context.as_ref().find_attr_value("duration") {
                          progress_bar.update_status(entity, "duration found", 0.01).await;
                          let start = Instant::now();
                          for i in 1..*duration + 1 {
                              sleep(Duration::from_secs(1)).await;
                              progress_bar.update_status(entity, format!("elapsed {:?}", start.elapsed()), i as f32/ (*duration as f32)).await;
                          }
                      } else {
                          sleep(Duration::from_secs(10)).await;
                      }
                      progress_bar.update_status(entity, "timer completed", 1.0).await;
                      thunk.call(&mut initial_context.clone());
                      ThunkContext::default()
                  })
              })
    }
}

impl Extension for Timer {
    fn configure_app_world(world: &mut World) {
        EventRuntime::configure_app_world(world);

        world.create_entity().with(Timer::event()).build();
    }

    fn configure_app_systems(dispatcher: &mut DispatcherBuilder) {
        EventRuntime::configure_app_systems(dispatcher);
    }

    fn on_ui(&'_ mut self, app_world: &World, ui: &'_ imgui::Ui<'_>) {
        ui.text(&self.1);
        if ui.button("fire") {
            self.0 = true;
        }

        if let Some(progress) = &self.2 {
            progress.show(ui);
        }
    }

    fn on_window_event(&'_ mut self, _: &World, _: &'_ WindowEvent<'_>) {
    }

    fn on_run(&'_ mut self, app_world: &World) {
        self.run_now(app_world);
        EventRuntime{}.on_run(app_world);
    }
}

impl<'a> System<'a> for Timer {
    type SystemData = (Entities<'a>, WriteStorage<'a, Event>, ReadStorage<'a, Progress>);

    fn run(&mut self, (entities, mut events, progress): Self::SystemData) {
        for (_, event) in (&entities, &mut events).join() {
            if self.0 {
                let mut initial = ThunkContext::default();
                initial.as_mut().with_int("duration", 5);
                event.fire(initial);
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

// ui.same_line();
// if ui.button("Compress state") {
//     use compression::prelude::*;
//     match std::fs::read(format!("{}.json", "projects")) {
//         Ok(serialized) => {
//             let compressed = serialized
//                 .encode(&mut BZip2Encoder::new(9), Action::Finish)
//                 .collect::<Result<Vec<_>, _>>()
//                 .unwrap();

//             if let Some(_) = std::fs::write("projects.json.bzip2", compressed).ok() {
//                 println!("compressed");
//             }
//         }
//         Err(_) => {}
//     }
// }

// ui.same_line();
// if ui.button("Decompress state") {
//     use compression::prelude::*;
//     match std::fs::read(format!("{}.json.bzip2", "projects")) {
//         Ok(compressed) => {
//             let decompressed = compressed
//                 .decode(&mut BZip2Decoder::new())
//                 .collect::<Result<Vec<_>, _>>()
//                 .unwrap();

//             if let Some(_) =
//                 std::fs::write("projects.json.bzip2.json", decompressed).ok()
//             {
//                 println!("decompressed");
//             }
//         }
//         Err(_) => {}
//     }
// }
