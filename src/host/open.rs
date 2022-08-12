use atlier::system::App;
use specs::System;

use super::{Dashboard, Host};

/// Opens the host's extension UI, Optionally, enable diagnostic dashboard
/// 
pub fn open(title: impl AsRef<str>, host: impl Host + 'static, enable_dashboard: bool) {
    if !enable_dashboard {
        crate::open::open(
            title.as_ref(), 
            Empty{}, 
            host
        );
    } else {
        crate::open::open(
            title.as_ref(), 
            Dashboard{}, 
            host
        );
    }
}

struct Empty{}

impl<'a> System<'a> for Empty {
    type SystemData = ();

    fn run(&mut self, _data: Self::SystemData) {
    }
}

impl App for Empty {
    fn name() -> &'static str {
        "empty"
    }

    fn edit_ui(&mut self, _ui: &imgui::Ui) {}

    fn display_ui(&self, _ui: &imgui::Ui) {}
}
