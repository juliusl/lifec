use specs::{System, ReadStorage, DispatcherBuilder};

use crate::prelude::*;
use super::Events;

pub fn install(dispatcher_builder: &mut DispatcherBuilder) {

}

/// Runtime is a system that finds 
/// 
struct Runtime; 

impl<'a> System<'a> for Runtime {
    type SystemData = (Events<'a>, ReadStorage<'a, Engine>);

    fn run(&mut self, (events, engines): Self::SystemData) {
        


    }
}