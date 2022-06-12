use crate::plugins::{self, Edit, Display};
use crate::{plugins::Plugin, AttributeGraph};

use super::ThunkContext;
use atlier::prelude::Value;
use atlier::system::Attribute;
use specs::{WorldExt, Builder, World, Component};
use specs::storage::DenseVecStorage;

#[derive(Component, Clone)]
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
        // if let Some(dir) = std::env::current_dir().ok() {
        //     let dir = dir.display().to_string();

        //     context.set_return::<WriteFiles>(Value::TextBuffer(dir));
        // }
    }
}

pub fn add_entity(initial: AttributeGraph, w: &mut World) {
    w.register::<AttributeGraph>();
    w.register::<WriteFiles>();
    w.register::<ThunkContext>();
    w.register::<Edit>();
    w.register::<Display>();

    w.create_entity()
     .with(initial)
     .maybe_with(Some(WriteFiles{}))
     .maybe_with(Some(ThunkContext(AttributeGraph::default())))
     .maybe_with(Some(Edit(|g, ui| {
         g.edit_attr_table(ui);
     })))
     .build();
}