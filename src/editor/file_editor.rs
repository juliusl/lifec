use std::collections::HashMap;
use std::fmt::Display;

use atlier::system::{App, Extension};
use imgui::Window;
use specs::storage::HashMapStorage;
use specs::{Component, Entities, Join, ReadStorage, RunNow, System};

use crate::RuntimeState;

use super::{SectionAttributes, SectionExtension};

#[derive(Component, Default)]
#[storage(HashMapStorage)]
pub struct FileEditor {
    files: HashMap<u32, FileEntry>,
}

#[derive(Debug, Default, Clone)]
pub struct FileEntry {
    file_name: String,
}

pub struct FileEntryRuntimeError {}

impl Display for FileEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "file_name: {}", self.file_name)
    }
}

impl RuntimeState for FileEntry {
    type Error = FileEntryRuntimeError;

    fn load<S: AsRef<str> + ?Sized>(&self, _: &S) -> Self
    where
        Self: Sized {
        todo!()
    }

    fn process<S: AsRef<str> + ?Sized>(&self, _: &S) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl Extension for FileEditor {
    fn configure_app_world(_: &mut specs::World) {}

    fn configure_app_systems(_: &mut specs::DispatcherBuilder) {}

    fn extend_app_world(&mut self, app_world: &specs::World, ui: &imgui::Ui) {
        self.run_now(app_world);
        self.show_editor(ui);
    }
}

impl SectionExtension<FileEntry> for FileEditor {
    fn show_extension(_: &mut super::Section<FileEntry>, _: &imgui::Ui) {
    }
}

impl App for FileEditor {
    fn name() -> &'static str {
        "File Editor"
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        Window::new("Files")
            .size([800.0, 600.0], imgui::Condition::Appearing)
            .build(ui, || {
                self.files
                    .iter()
                    .for_each(|(_, file_entry)| ui.text(format!("{:?}", file_entry.file_name)));
            });
    }
}

impl<'a> System<'a> for FileEditor {
    type SystemData = (Entities<'a>, ReadStorage<'a, SectionAttributes>);

    fn run(&mut self, (entities, section_attributes): Self::SystemData) {
        for e in entities.join() {
            if let Some(attrs) = section_attributes.get(e) {
                match attrs.is_attr_checkbox("enable file editor") {
                    Some(true) => {
                        let mut file_name = None;
                        attrs
                            .get_attrs()
                            .iter()
                            .filter(|a| a.name().starts_with("file::"))
                            .map(|a| a.name())
                            .for_each(|a| {
                                if let true = a.starts_with("file::name::") {
                                    let value = &a["file::name::".len()..];
                                    file_name = Some(value);
                                }
                            });
                        if let Some(file_name) = file_name.and_then(|s| Some(s.to_string())) {
                            self.files.insert(e.id(), FileEntry { file_name });
                        }
                    }
                    Some(false) => {
                        self.files.remove(&e.id());
                    }
                    _ => {}
                }
            }
        }
    }
}
