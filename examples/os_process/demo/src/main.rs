use lifec::{plugins::*, editor::{*, runtime_editor::Receiver}, Runtime, AttributeGraph};

#[derive(Default)]
struct Demo(RuntimeEditor, Option<Receiver<Entity>>);

impl Extension for Demo {
    fn configure_app_world(world: &mut World) {
        RuntimeEditor::configure_app_world(world);
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

        let Demo(editor, ..) = self;
        editor.on_ui(app_world, ui);

        if let Some(Value::Bool(show_demo_window)) = self.0.project_mut().as_mut().find_attr_value_mut("show_demo_window") {
            ui.show_demo_window(show_demo_window);
        }
    }

    fn on_window_event(&'_ mut self, app_world: &World, event: &'_ WindowEvent<'_>) {
        let Demo(editor, ..) = self;
        editor.on_window_event(app_world, event);
    }

    fn on_run(&'_ mut self, world: &World) {
        self.0.on_run(world);
    }  
}

fn main() {
    if let Some(file) = AttributeGraph::load_from_file("drag_drop_example.runmd") {
        let mut runtime = Runtime::new(Project::from(file.clone()));
        runtime.install::<Call, Timer>();
        runtime.install::<Call, Process>();
        runtime.install::<Call, OpenFile>();
        runtime.install::<Call, OpenDir>();
        let mut demo = Demo::default();
        *demo.0.project_mut().as_mut() = file;
        demo.0.project_mut().reload_source();
        open(
            "demo",
            runtime,
            demo,
        );
    }
}