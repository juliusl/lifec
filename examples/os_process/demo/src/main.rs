use std::time::Duration;

use lifec::plugins::{Event, EventRuntime, Node, Plugin, ThunkContext};
use lifec::{editor::*, AttributeGraph, Runtime};
use specs::storage::DenseVecStorage;
use specs::{
    Component, DispatcherBuilder, Entities, Join, ReadStorage, RunNow, System, World, WriteStorage, Read,
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
struct Timer(bool, String);

impl Plugin<ThunkContext> for Timer {
    fn symbol() -> &'static str {
        "timer"
    }

    fn call_with_context(context: &mut ThunkContext) {
        println!("timer finished");
    }
}

impl Extension for Timer {
    fn configure_app_world(world: &mut World) {
        EventRuntime::configure_app_world(world);

        let event =
            Event::from_plugin_with::<Self>("start_timer", |thunk, initial_context, handle| {
                let thunk = thunk.clone();
                let initial_context = initial_context.clone();
                handle.spawn(async move {
                    println!("timer started");

                    if let Some(Value::Int(duration)) = initial_context.as_ref().find_attr_value("duration") {
                        println!("duration setting found {}", duration);
                        sleep(Duration::from_secs(*duration as u64)).await;
                    } else {
                        sleep(Duration::from_secs(10)).await;
                    }

                    thunk.call(&mut initial_context.clone());
                    ThunkContext::default()
                })
            });

        world.create_entity().with(event).build();
    }

    fn configure_app_systems(dispatcher: &mut DispatcherBuilder) {
        EventRuntime::configure_app_systems(dispatcher);
    }

    fn on_ui(&'_ mut self, app_world: &World, ui: &'_ imgui::Ui<'_>) {
        ui.text(&self.1);
        if ui.button("fire") {
            self.0 = true;
        }
        self.run_now(app_world);
    }
}

impl<'a> System<'a> for Timer {
    type SystemData = (Entities<'a>, WriteStorage<'a, Event>);

    fn run(&mut self, (entities, mut events): Self::SystemData) {
        for (entity, event) in (&entities, &mut events).join() {
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
