use specs::System;


pub struct Event;

impl<'a> System<'a> for Event {
    type SystemData = ();

    fn run(&mut self, data: Self::SystemData) {
        todo!()
    }
}