use atlier::system::App;
use specs::System;


pub struct Dashboard {
 
}

impl<'a> System<'a> for Dashboard {
    type SystemData = ();

    fn run(&mut self, data: Self::SystemData) {
        todo!()
    }
}

impl App for Dashboard {
    fn name() -> &'static str {
        "dashboard"
    }

    fn edit_ui(&mut self, ui: &imgui::Ui) {
        todo!()
    }

    fn display_ui(&self, ui: &imgui::Ui) {
        todo!()
    }
}