use crate::prelude::{ErrorContext, SecureClient, StatusUpdate};
use crate::{
    engine::{Completion, Runner, Yielding},
    guest::Guest,
    prelude::*,
};
use hyper::Client;
use hyper_tls::HttpsConnector;
use specs::Write;
use specs::{shred::SetupHandler, Entity, System, World};
use tokio::sync::{self, broadcast, mpsc};

use super::Journal;

/// Event runtime drives the tokio::Runtime and schedules/monitors/orchestrates plugin events
///
#[derive(Default)]
pub struct EventRuntime;

/// Setup for tokio runtime, (Not to be confused with crate::Runtime)
impl SetupHandler<tokio::runtime::Runtime> for EventRuntime {
    fn setup(world: &mut specs::World) {
        if !world.has_value::<tokio::runtime::Runtime>() {
            world.insert(tokio::runtime::Runtime::new().unwrap());
        }
    }
}

/// Setup for watch channel for host editor
impl SetupHandler<sync::watch::Receiver<HostEditor>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        if !world.has_value::<sync::watch::Receiver<HostEditor>>() {
            let (tx, rx) = sync::watch::channel::<HostEditor>(HostEditor::default());
            world.insert(rx);
            world.insert(tx);
        }
    }
}

/// Setup for watch channel for host editor
impl SetupHandler<sync::watch::Sender<HostEditor>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        if !world.has_value::<sync::watch::Sender<HostEditor>>() {
            let (tx, rx) = sync::watch::channel::<HostEditor>(HostEditor::default());
            world.insert(rx);
            world.insert(tx);
        }
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for guests
impl SetupHandler<sync::mpsc::Sender<Guest>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        if !world.has_value::<sync::mpsc::Sender<Guest>>() {
            let (tx, rx) = mpsc::channel::<Guest>(30);
            world.insert(tx);
            world.insert(rx);
        }
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for guests
impl SetupHandler<sync::mpsc::Receiver<Guest>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        if !world.has_value::<sync::mpsc::Receiver<Guest>>() {
            let (tx, rx) = mpsc::channel::<Guest>(30);
            world.insert(tx);
            world.insert(rx);
        }
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for completion
impl SetupHandler<sync::mpsc::Sender<Completion>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        if !world.has_value::<sync::mpsc::Sender<Completion>>() {
            let (tx, rx) = mpsc::channel::<Completion>(30);
            world.insert(tx);
            world.insert(rx);
        }
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for completion
impl SetupHandler<sync::mpsc::Receiver<Completion>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        if !world.has_value::<sync::mpsc::Receiver<Completion>>() {
            let (tx, rx) = mpsc::channel::<Completion>(30);
            world.insert(tx);
            world.insert(rx);
        }
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for nodes
impl SetupHandler<sync::mpsc::Sender<(NodeCommand, Option<Yielding>)>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        if !world.has_value::<sync::mpsc::Sender<(NodeCommand, Option<Yielding>)>>() {
            let (tx, rx) = mpsc::channel::<(NodeCommand, Option<Yielding>)>(30);
            world.insert(tx);
            world.insert(rx);
        }
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for nodes
impl SetupHandler<sync::mpsc::Receiver<(NodeCommand, Option<Yielding>)>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        if !world.has_value::<sync::mpsc::Receiver<(NodeCommand, Option<Yielding>)>>() {
            let (tx, rx) = mpsc::channel::<(NodeCommand, Option<Yielding>)>(30);
            world.insert(tx);
            world.insert(rx);
        }
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for status updates
impl SetupHandler<sync::mpsc::Sender<StatusUpdate>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        if !world.has_value::<sync::mpsc::Sender<StatusUpdate>>() {
            let (tx, rx) = mpsc::channel::<StatusUpdate>(30);
            world.insert(tx);
            world.insert(rx);
        }
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for status updates
impl SetupHandler<sync::mpsc::Receiver<StatusUpdate>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        if !world.has_value::<sync::mpsc::Receiver<StatusUpdate>>() {
            let (tx, rx) = mpsc::channel::<StatusUpdate>(30);
            world.insert(tx);
            world.insert(rx);
        }
    }
}

/// Setup for tokio-broadcast channel for entity updates
impl SetupHandler<sync::broadcast::Receiver<Entity>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        if !world.has_value::<sync::broadcast::Receiver<Entity>>() {
            let (tx, rx) = broadcast::channel::<Entity>(100);
            world.insert(rx);
            world.insert(tx);
        }
    }
}

/// Setup for tokio-broadcast channel for entity updates
impl SetupHandler<sync::broadcast::Sender<Entity>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        if !world.has_value::<sync::broadcast::Sender<Entity>>() {
            let (tx, rx) = broadcast::channel::<Entity>(100);
            world.insert(rx);
            world.insert(tx);
        }
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for status updates
impl SetupHandler<sync::mpsc::Sender<ErrorContext>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        if !world.has_value::<sync::mpsc::Sender<ErrorContext>>() {
            let (tx, rx) = mpsc::channel::<ErrorContext>(10);
            world.insert(tx);
            world.insert(rx);
        }
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for status updates
impl SetupHandler<sync::mpsc::Receiver<ErrorContext>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        if !world.has_value::<sync::mpsc::Receiver<ErrorContext>>() {
            let (tx, rx) = mpsc::channel::<ErrorContext>(10);
            world.insert(tx);
            world.insert(rx);
        }
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for status updates
impl SetupHandler<sync::mpsc::Receiver<Operation>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        if !world.has_value::<sync::mpsc::Receiver<Operation>>() {
            let (tx, rx) = mpsc::channel::<Operation>(10);
            world.insert(tx);
            world.insert(rx);
        }
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for status updates
impl SetupHandler<sync::mpsc::Sender<Operation>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        if !world.has_value::<sync::mpsc::Sender<Operation>>() {
            let (tx, rx) = mpsc::channel::<Operation>(10);
            world.insert(tx);
            world.insert(rx);
        }
    }
}

/// Setup for a built-in runtime for the world
impl SetupHandler<Runtime> for EventRuntime {
    fn setup(world: &mut World) {
        if !world.has_value::<Runtime>() {
            world.insert(Runtime::default());
        }
    }
}

/// Setup for a shared https client
impl SetupHandler<SecureClient> for EventRuntime {
    fn setup(world: &mut World) {
        if !world.has_value::<SecureClient>() {
            let https = HttpsConnector::new();
            let client = Client::builder().build::<_, hyper::Body>(https);
            world.insert(client);
        }
    }
}

impl<'a> System<'a> for EventRuntime {
    type SystemData = (
        State<'a>,
        Runner<'a>,
        Write<'a, Journal>,
    );

    fn run(&mut self, (mut events, mut runner, mut journal): Self::SystemData) {
        if !events.should_exit() && events.can_continue() {
            events.tick();
        } else {
            // If a rate limit is set, this will update the freq w/o changing the last tick
            events.handle_rate_limits();
        }

        // Handle any node commands,
        for (entity, command) in runner.take_commands() {
            if events.handle_node_command(command.clone()) {
                journal.push((entity, command));
            }
        }

        // Run guests
        for guest in runner.guests() {
            guest.run();
            guest.maintain();
        }
    }
}
