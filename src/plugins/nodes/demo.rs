use atlier::system::Extension;
use specs::Builder;

use crate::plugins::{Node, Plugin, Edit, Display};

use super::NodeContext;

#[derive(Default)]
pub struct NodeDemo(Node);

impl Extension for NodeDemo {
    fn configure_app_world(world: &mut specs::World) {
        Node::configure_app_world(world);

        Node::add_node_from(".runmd", world, |e|{
            let display = Display::<NodeContext>(
                |c, g, ui|{
                    ui.text("hello");
                });

            e.maybe_with(Some(display)).build()
        });
    }

    fn configure_app_systems(dispatcher: &mut specs::DispatcherBuilder) {
        Node::configure_app_systems(dispatcher);
    }

    fn on_ui(&mut self, app_world: &specs::World, ui: &imgui::Ui<'_>) {
        self.0.on_ui(app_world, ui)
    }
}
