use lifec::{plugins::*, editor::*, AttributeGraph, Runtime};

fn main() {
    if let Some(file) = AttributeGraph::load_from_file("test_demo.runmd") {
        let runtime = Runtime::new(Project::from(file));
        open(
            "demo",
            runtime,
            Demo::default(),
        );
    }
}

#[derive(Default)]
struct Demo(RuntimeEditor);

impl Extension for Demo {
    fn configure_app_world(world: &mut World) {
        RuntimeEditor::configure_app_world(world);
        Call::init::<Timer>(world, |config| {
            config.as_mut().add_int_attr("duration", 5);
        });
        
        Call::init::<Process>(world, |config| {
            config.as_mut().add_text_attr("command", "cargo help");
        });

        Call::init::<OpenFile>(world, |config| {
            config.as_mut().add_text_attr("file_src", "test_demo.runmd");
        });
    }

    fn configure_app_systems(dispatcher: &mut DispatcherBuilder) {
        RuntimeEditor::configure_app_systems(dispatcher);
    }

    fn on_ui(&'_ mut self, app_world: &World, ui: &'_ imgui::Ui<'_>) {
        if ui.button("create new process") {
            Call::create::<Process>(app_world, |c| {
                c.as_mut().add_text_attr("command", "echo hello world");
            });
        }

        let Demo(editor) = self;
        editor.on_ui(app_world, ui);
    }

    fn on_window_event(&'_ mut self, app_world: &World, event: &'_ WindowEvent<'_>) {
        let Demo(editor) = self;
        editor.on_window_event(app_world, event);
    }

    fn on_run(&'_ mut self, _: &World) {
    }
}

