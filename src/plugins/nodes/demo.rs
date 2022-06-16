use atlier::system::Extension;
use specs::{Builder, WorldExt};

use crate::plugins::{Node, Plugin, Edit, Display, ThunkContext, WriteFiles, demos::WriteFilesDemo};

use super::NodeContext;

/// Starts a demo of the node editor
pub struct NodeDemo(Node);

impl Default for NodeDemo
{
    fn default() -> Self {
        Self(Node::new())
    }
}

impl Extension for NodeDemo
{
    fn configure_app_world(world: &mut specs::World) {
        world.register::<ThunkContext>();
        world.register::<WriteFiles>();
        world.register::<Edit::<ThunkContext>>();

        Node::configure_app_world(world);
        Node::parse_entity(".runmd", world, |e|{
            let display = Display::<NodeContext>(
                |_, _, ui|{
                    ui.text("hello");
                });

            e.maybe_with(Some(display))
             .build()
        });

        WriteFiles::parse_entity("println.runmd", world, |e|{
            let edit = Edit::<ThunkContext>(
                |_, g, ui| {
                    g.edit_attr_table(ui);
                }
            );

            e.maybe_with(Some(edit))
             .build()
        });

        WriteFilesDemo::configure_app_world(world);
    }

    fn configure_app_systems(dispatcher: &mut specs::DispatcherBuilder) {
        Node::configure_app_systems(dispatcher);
        WriteFilesDemo::configure_app_systems(dispatcher);
    }

    fn on_ui(&mut self, app_world: &specs::World, ui: &imgui::Ui<'_>) {
        // Starts the node editor
        self.0.on_ui(app_world, ui);

        // Since write files demo is stateless, it doesn't need to maintain state between frames
        WriteFilesDemo::default()
            .on_ui(app_world, ui);
    }
}
