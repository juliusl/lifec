use lifec::{plugins::*, editor::*, AttributeGraph};

#[derive(Default)]
struct Demo(RuntimeEditor);

impl Extension for Demo {
    fn configure_app_world(world: &mut World) {
        RuntimeEditor::configure_app_world(world);
    }

    fn configure_app_systems(dispatcher: &mut DispatcherBuilder) {
        RuntimeEditor::configure_app_systems(dispatcher);
    }

    fn on_ui(&'_ mut self, app_world: &World, ui: &'_ imgui::Ui<'_>) {
        let Demo(editor, ..) = self;
        editor.on_ui(app_world, ui);

        if let Some(Value::Bool(show_demo_window)) = self.0.project_mut().as_mut().find_attr_value_mut("show_demo_window") {
            ui.show_demo_window(show_demo_window);
        }
    }

    fn on_window_event(&'_ mut self, app_world: &World, event: &'_ WindowEvent<'_>) {
        let Demo(editor, ..) = self;
        editor.on_window_event(app_world, event);

        match event {
            WindowEvent::DroppedFile(path) => {
                if "runmd" == path.extension().unwrap_or_default() {
                    if let Some(file) = AttributeGraph::load_from_file(path.to_str().unwrap_or_default()) {
                        let mut demo = Demo::default();
                        *demo.0.project_mut().as_mut() = file;
                        let demo_project = demo.0.project_mut().reload_source();
                        *demo.0.project_mut() = demo_project;
                        self.0.runtime_mut().read_project(app_world);
                    }
                }
            },
            _ => {

            }
        }
    }

    fn on_run(&'_ mut self, world: &World) {
        self.0.on_run(world);
    }  
}

fn main() {
    if let Some(file) = AttributeGraph::load_from_file("drag_drop_example.runmd") {
        let mut demo = Demo::default();
        *demo.0.project_mut().as_mut() = file;
        let demo_project = demo.0.project_mut().reload_source();
        *demo.0.project_mut() = demo_project;

        demo.0.runtime_mut().add_config(Config("timer_simple", |tc| {
            tc.block.block_name = unique_title("simple");
            tc.as_mut().with_int_range("duration", &[1, 0, 10]);
        }));

        demo.0.runtime_mut().add_config(Config("timer_complex", |tc| {
            tc.block.block_name = unique_title("complex");
            tc.as_mut().with_int_range("duration", &[1, 0, 10]);
            tc.as_mut().with_float_range("duration_ms", &[0.0, 0.0, 1000.0]);
        }));

        demo.0.runtime_mut().add_config(Config("cargo_run", |tc| {
            tc.block.block_name = unique_title("cargo_run");
            tc.as_mut().with_text("command", "cargo run .");
        }));

        open(
            "Demo",
            Demo::default(),
            demo,
        );
    }
}

impl App for Demo {
    fn name() -> &'static str {
        "demo"
    }

    fn edit_ui(&mut self, _: &imgui::Ui) {
    }

    fn display_ui(&self, _: &imgui::Ui) {
    }
}

impl<'a> System<'a> for Demo {
    type SystemData = (
    );

    fn run(&mut self, _: Self::SystemData) {
   
    }
}