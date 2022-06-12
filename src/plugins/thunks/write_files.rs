use crate::plugins::{Display, Edit};
use crate::{plugins::Plugin, AttributeGraph};

use super::ThunkContext;
use atlier::prelude::Value;
use imgui::Window;
use specs::storage::DenseVecStorage;
use specs::{Builder, Component, World, WorldExt};

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
        context
            .as_ref()
            .iter_attributes()
            .map(|a| (a.name(), a.value()))
            .for_each(|(file_name, value)| {
                if let Value::BinaryVector(content) = value {
                    let path = file_name.replace("::", ".");
                    if let Err(err) = std::fs::write(&path, content) {
                        eprintln!("did not write file {}, {}", file_name, err);
                    }
                } else {
                    eprintln!("skipping write file for: {:?}", (file_name, value));
                }
            });

        // returns current directory
        if let Some(dir) = std::env::current_dir().ok() {
            let dir = dir.display().to_string();

            println!("Setting parent dir {}", dir);
            context.write_output("parent_dir", Value::TextBuffer(dir));
        }
    }
}


pub fn demo_write_files(initial: AttributeGraph, w: &mut World) {
    w.register::<AttributeGraph>();
    w.register::<WriteFiles>();
    w.register::<ThunkContext>();
    w.register::<Edit<ThunkContext>>();
    w.register::<Display<ThunkContext>>();

    w.create_entity()
        .with(initial)
        .maybe_with(Some(ThunkContext(
            AttributeGraph::load_from_file(".runmd").unwrap_or_default(),
        )))
        .maybe_with(Some(Edit::<ThunkContext>(|initial, g, ui| {
            Window::new("demo").build(ui, || {
                if ui.button("write all files") {
                    WriteFiles::call(g);
                }
                initial.as_mut().edit_attr_table(ui);
                g.edit_attr_table(ui);
            });
        })))
        .build();
}
