use crate::plugins::Plugin;
use super::ThunkContext;
use atlier::prelude::Value;
use specs::storage::DenseVecStorage;
use specs::Component;

#[derive(Component, Clone, Default)]
#[storage(DenseVecStorage)]
pub struct WriteFiles;

impl Plugin<ThunkContext> for WriteFiles {
    fn symbol() -> &'static str {
        "write_files"
    }

    fn description() -> &'static str {
        "Writes any input binary vector value to a file."
    }

    fn call_with_context(context: &mut ThunkContext) {
        for (file_name, value) in context
            .clone()
            .as_ref()
            .iter_attributes()
            .map(|a| (a.name(), a.value()))
            .filter_map(|(file_name, value)| {
                if let Value::BinaryVector(content) = value {
                    let path = file_name.replace("::", ".");
                    if let Err(err) = std::fs::write(&path, content) {
                        eprintln!("did not write file {}, {}", file_name, err);
                    } else {
                        return Some((file_name, Value::TextBuffer(path)));
                    }
                } else {
                    eprintln!("skipping write file for: {:?}", (file_name, value));
                }
                None
            })
        {
            context.publish(|a| { 
                a.with(file_name, value);
            });
        }

        // returns current directory
        if let Some(dir) = std::env::current_dir().ok() {
            let dir = dir.display().to_string();

            println!("Setting parent dir {}", dir);
            context.publish(|a| a.add_text_attr("parent_dir", dir));
        }
    }
}

pub mod demo {
    use atlier::system::Extension;
    use imgui::Window;
    use specs::{Builder, WorldExt};

    use crate::{
        plugins::{Display, Edit, ThunkContext, Plugin, Render},
        AttributeGraph,
    };

    use super::WriteFiles;

    #[derive(Default)]
    pub struct WriteFilesDemo;

    impl Extension for WriteFilesDemo {
        fn configure_app_world(world: &mut specs::World) {
            world.register::<AttributeGraph>();
            world.register::<WriteFiles>();
            world.register::<ThunkContext>();
            world.register::<Edit<ThunkContext>>();
            world.register::<Display<ThunkContext>>();

            WriteFiles::parse_entity("demo.runmd", world, |e| {
                e.maybe_with(Some(ThunkContext::from(
                    AttributeGraph::load_from_file(".runmd").unwrap_or_default(),
                )))
                .maybe_with(Some(Edit::<ThunkContext>(|initial, g, ui| {
                    Window::new("demo").build(ui, || {
                        if ui.button("write all files") {
                            WriteFiles::call(g);
                        }
                        initial.clone()
                            .as_mut()
                            .edit_attr_table(ui);
                            
                        g.edit_attr_table(ui);
                    });
                })))
                .build()
            });
        }

        fn configure_app_systems(_: &mut specs::DispatcherBuilder) {
        }


        fn on_ui(&mut self, app_world: &specs::World, ui: &imgui::Ui) {
            let mut render = Render::<ThunkContext>::next_frame(ui);
            render.render_now(app_world);
        }
    }
}