use specs::{Entities, Join, ReadStorage, System, WriteStorage};
use super::{NextButton, StartButton};

/// Generates a next button component for all start buttons
pub struct Sequencer;

impl<'a> System<'a> for Sequencer {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, StartButton>,
        WriteStorage<'a, NextButton>,
    );

    fn run(&mut self, (entities, start_buttons, mut next_buttons): Self::SystemData) {
        for (entity, event) in (&entities, start_buttons.maybe()).join() {
            if let Some(event) = event {
                if !next_buttons.contains(entity) {
                    match next_buttons.insert(entity, NextButton(event.clone(), None, None)) {
                        Ok(_) => {
                            eprintln!("adding next button for {:?}", entity);
                        }
                        Err(err) => {
                            eprintln!("sequencer could not add next_button, {}", err);
                        }
                    }
                }
            }
        }
    }
}
