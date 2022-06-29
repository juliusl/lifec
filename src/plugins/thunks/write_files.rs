use crate::plugins::Plugin;
use super::ThunkContext;
use atlier::prelude::Value;
use specs::storage::DenseVecStorage;
use specs::Component;
use tokio::task::JoinHandle;

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

    fn call_with_context(context: &mut ThunkContext) -> Option<JoinHandle<ThunkContext>> {
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

        None
    }
}
