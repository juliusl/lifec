use super::ThunkContext;
use super::Thunk;
use atlier::prelude::Value;

pub struct WriteFiles;

impl Thunk for WriteFiles {
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

            context.set_return::<WriteFiles>(Value::TextBuffer(dir));
        }
    }
}