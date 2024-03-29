use specs::{World, DispatcherBuilder, WorldExt};
use tracing::{event, Level};

use crate::{Extension, Runtime, editor::Call, plugins::{Event, ThunkContext}};

/// start creates an engine from the runtime, and begins the world in a loop
pub fn start<E, S>(mut extension: E, call_sequence: Vec<S>)
where
    E: Extension + AsRef<Runtime> + 'static,
    S: AsRef<str> + ToString,
{
    let mut world = World::new();
    let mut dipatch_builder = DispatcherBuilder::new();
   
    E::configure_app_world(&mut world);
    E::configure_app_systems(&mut dipatch_builder);
    
    let mut dispatcher = dipatch_builder.build();
    dispatcher.setup(&mut world);
    
    for sequence_name in call_sequence {
        if let Some(start) = extension.as_ref().create_engine::<Call>(&world, sequence_name.to_string()) {
            event!(Level::INFO, "Created engine {:?}", start);
    
            let mut event = world.write_component::<Event>();
            let tc = world.read_component::<ThunkContext>();
            let event = event.get_mut(start);
            if let Some(event) = event {
                if let Some(tc) = tc.get(start) {
                    event.fire(tc.clone());
                }
            }
        }
    }
    
    loop {
        // TODO, use a tokio thread instead
        dispatcher.dispatch(&world);
        extension.on_run(&world);
    
        world.maintain();
        extension.on_maintain(&mut world);
    }
}