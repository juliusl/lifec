use std::collections::HashMap;

use atlier::system::Extension;
use specs::Entity;

use crate::{host::Host, plugins::Project, Runtime};

// mod cube;
// use cube::Cube;

// mod node;
// use node::Node;

mod engine;
use engine::ProtocolEngine;

/// Protocol is a host implementation that
/// manages a set of internal engines operating with a guest runtime
///
#[derive(Default)]
pub struct Protocol {
    plugins: Vec<fn(&mut Runtime)>,
    runtimes: HashMap<Entity, Runtime>,
    hosts: HashMap<Entity, ProtocolEngine>,
}

impl Protocol {
    /// Adds a new plugin install function to the protocol
    ///
    pub fn add_plugin(&mut self, plugin: fn(&mut Runtime)) {
        self.plugins.push(plugin);
    }
}

impl Host for Protocol {
    fn create_runtime(&mut self, project: Project) -> Runtime {
        let mut runtime = Runtime::new(project);

        // Apply plugins
        for p in self.plugins.iter() {
            p(&mut runtime);
        }

        runtime
    }

    fn get_runtime(&mut self, engine: Entity) -> Runtime {
        self.runtimes
            .get(&engine)
            .expect("runtime should be available")
            .clone()
    }

    fn add_runtime(&mut self, engine: Entity, runtime: Runtime) {
        self.runtimes.insert(engine, runtime);
    }

    /// Adds a new guest runtime to the protocol
    ///
    fn add_guest(&mut self, host: specs::Entity, dispatcher: specs::Dispatcher<'static, 'static>) {
        let engine = ProtocolEngine::new(self.get_runtime(host), dispatcher);
        self.hosts.insert(host, engine);
    }

    /// Activates the protocol engine for the guest runtime
    ///
    fn activate_guest(
        &mut self,
        host: specs::Entity,
    ) -> Option<specs::Dispatcher<'static, 'static>> {
        self.hosts.get_mut(&host).and_then(ProtocolEngine::activate)
    }

    /// Should exit if
    ///
    fn should_exit(&mut self) -> Option<crate::host::HostExitCode> {
        todo!()
    }
}

impl Extension for Protocol {
    fn on_run(&'_ mut self, _app_world: &specs::World) {}

    fn on_maintain(&'_ mut self, _app_world: &mut specs::World) {}
}

#[test]
fn test_protocol() {
    let mut protocol = Protocol::default();

    protocol.add_plugin(|r| {
        r.install::<crate::plugins::Test, crate::plugins::Println>();
    });

    protocol.start(
        Project::load_content(
            r#"
    ``` protocol test 
    ```
    "#,
        )
        .expect("valid .runmd"),
    );
}
