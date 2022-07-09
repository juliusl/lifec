use specs::{System, ReadStorage, Join};

use crate::{editor::ProgressStatusBar, plugins::ThunkContext};


pub struct Trace;

impl<'a> System<'a> for Trace {
    type SystemData = (
        ReadStorage<'a, ThunkContext>,
        ReadStorage<'a, ProgressStatusBar>
    );

    fn run(&mut self, (contexts, tasks): Self::SystemData) {
        for (tc, progress) in (&contexts, &tasks).join() {
            let block_name = &tc.block.block_name;

            let ProgressStatusBar(p, label, status, _) = progress;

            eprint!("{block_name}: {label} ");
            eprint!("{p}% ");
            eprintln!("{}", status.lines().last().unwrap_or_default());

        }
    }
}