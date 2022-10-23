use specs::prelude::*;
use tokio::sync::mpsc::Receiver;

use crate::prelude::*;

/// Resources for consuming messages from plugins,
///
/// Can only be a single consumer per world,
///
#[derive(SystemData)]
pub struct Listener<'a>(
    Write<'a, Receiver<StatusUpdate>, EventRuntime>,
    Write<'a, Receiver<RunmdFile>, EventRuntime>,
    Write<'a, Receiver<Operation>, EventRuntime>,
    Write<'a, Receiver<Start>, EventRuntime>,
);
