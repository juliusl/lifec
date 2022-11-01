use super::thunks::{ErrorContext, SecureClient, StatusUpdate};
use crate::{guest::Guest, prelude::*, engine::Yielding};
use hyper::Client;
use hyper_tls::HttpsConnector;
use specs::{shred::SetupHandler, Entity, System, World};
use tokio::sync::{self, broadcast, mpsc};

/// Event runtime drives the tokio::Runtime and schedules/monitors/orchestrates plugin events
///
#[derive(Default)]
pub struct EventRuntime;

/// Setup for tokio runtime, (Not to be confused with crate::Runtime)
impl SetupHandler<tokio::runtime::Runtime> for EventRuntime {
    fn setup(world: &mut specs::World) {
        world.insert(tokio::runtime::Runtime::new().unwrap());

        // TODO: setup shutdown hook
    }
}

/// Setup for watch channel for host editor
impl SetupHandler<sync::watch::Receiver<HostEditor>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = sync::watch::channel::<HostEditor>(HostEditor::default());
        world.insert(rx);
        world.insert(tx);
    }
}

/// Setup for watch channel for host editor
impl SetupHandler<sync::watch::Sender<HostEditor>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = sync::watch::channel::<HostEditor>(HostEditor::default());
        world.insert(rx);
        world.insert(tx);
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for guests
impl SetupHandler<sync::mpsc::Sender<Guest>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<Guest>(30);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for guests
impl SetupHandler<sync::mpsc::Receiver<Guest>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<Guest>(30);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for nodes
impl SetupHandler<sync::mpsc::Sender<(NodeCommand, Option<Yielding>)>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<(NodeCommand, Option<Yielding>)>(30);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for nodes
impl SetupHandler<sync::mpsc::Receiver<(NodeCommand, Option<Yielding>)>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<(NodeCommand, Option<Yielding>)>(30);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for status updates
impl SetupHandler<sync::mpsc::Sender<StatusUpdate>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<StatusUpdate>(30);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for status updates
impl SetupHandler<sync::mpsc::Receiver<StatusUpdate>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<StatusUpdate>(30);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for tokio-broadcast channel for entity updates
impl SetupHandler<sync::broadcast::Receiver<Entity>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = broadcast::channel::<Entity>(100);
        world.insert(rx);
        world.insert(tx);
    }
}

/// Setup for tokio-broadcast channel for entity updates
impl SetupHandler<sync::broadcast::Sender<Entity>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = broadcast::channel::<Entity>(100);
        world.insert(rx);
        world.insert(tx);
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for status updates
impl SetupHandler<sync::mpsc::Sender<RunmdFile>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<RunmdFile>(10);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for status updates
impl SetupHandler<sync::mpsc::Receiver<RunmdFile>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<RunmdFile>(10);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for status updates
impl SetupHandler<sync::mpsc::Sender<ErrorContext>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<ErrorContext>(10);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for status updates
impl SetupHandler<sync::mpsc::Receiver<ErrorContext>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<ErrorContext>(10);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for status updates
impl SetupHandler<sync::mpsc::Receiver<Operation>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<Operation>(10);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for status updates
impl SetupHandler<sync::mpsc::Sender<Operation>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<Operation>(10);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for host start command
impl SetupHandler<sync::mpsc::Receiver<Start>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<Start>(10);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for host start command
impl SetupHandler<sync::mpsc::Sender<Start>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<Start>(10);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for a built-in runtime for the world
impl SetupHandler<super::Runtime> for EventRuntime {
    fn setup(world: &mut World) {
        world.insert(super::Runtime::default());
    }
}

/// Setup for a shared https client
impl SetupHandler<SecureClient> for EventRuntime {
    fn setup(world: &mut World) {
        let https = HttpsConnector::new();
        let client = Client::builder().build::<_, hyper::Body>(https);
        world.insert(client);
    }
}

impl<'a> System<'a> for EventRuntime {
    type SystemData = (
        Entities<'a>, 
        Events<'a>, 
        WriteStorage<'a, NodeCommand>
    );

    fn run(&mut self, (entities, mut events, mut commands): Self::SystemData) {
        if !events.should_exit() && events.can_continue() {
            events.tick();
        } else {
            // If a rate limit is set, this will update the freq w/o changing the last tick
            events.handle_rate_limits();
        }

        // Handle any node commands,
        for (_, command) in (&entities, commands.drain()).join() {
            events.handle_node_command(command);
        }
    }
}
