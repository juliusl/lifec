use std::env;

use lifec::{editor::*, plugins::*, AttributeGraph, open, start, Runtime, Extension, World, DispatcherBuilder, System};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

/// Demo app for the runtime, can swap projects by dropping a .runmd file in
#[derive(Default)]
struct Demo(RuntimeEditor, bool);

impl Demo {
    fn new() -> Self {
        let mut demo = Demo::default();
        demo.0
            .runtime_mut()
            .add_config(Config("timer_simple", |tc| {
                tc.block.block_name = unique_title("simple");
                tc.as_mut()
                    .with_int_range("duration", &[1, 0, 10]);
            }));
    
        demo.0
            .runtime_mut()
            .add_config(Config("timer_complex", |tc| {
                tc.block.block_name = unique_title("complex");
                tc.as_mut()
                    .with_int_range("duration", &[1, 0, 10])
                    .add_float_range_attr("duration_ms", &[0.0, 0.0, 1000.0]);
            }));
    
        demo.0
            .runtime_mut()
            .add_config(Config("cargo_run", |tc| {
                tc.block.block_name = unique_title("cargo_run");
                tc.as_mut()
                    .with_text("command", "cargo run .");
            }));
        demo
    }

    fn load(file: AttributeGraph) -> Self {
        let mut demo = Self::new();
        *demo.0.project_mut().as_mut() = file;
        let demo_project = demo.0.project_mut().reload_source();
        *demo.0.project_mut() = demo_project;

        demo
    }
}

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

        imgui::Window::new("Combo example").size([800.0, 600.0], imgui::Condition::Appearing).build(ui, || {
            if let Some(block) = self.0.project_mut().find_block_mut("test") {
                block.update_block("combo", |a| {
                    a.combo_box("combo test", "combo_test", ui);
                });

                block.get_block("combo")
                    .unwrap_or_default()
                    .edit_attr_custom("combo_test_choice", |attr|{
                        attr.edit_value("chosen", ui);
                    });
             }
        });
    }

    fn on_window_event(&'_ mut self, app_world: &World, event: &'_ WindowEvent<'_>) {
        match event {
            WindowEvent::DroppedFile(path) => {
                if "runmd" == path.extension().unwrap_or_default() {
                    if let Some(file) =
                        AttributeGraph::load_from_file(path.to_str().unwrap_or_default())
                    {
                        *self.0.project_mut().as_mut() = file;
                        *self.0.project_mut() = self.0.project_mut().reload_source();
                        self.1 = true;
                    }
                }
            }
            _ => {}
        }

        let Demo(editor, ..) = self;
        editor.on_window_event(app_world, event);
    }

    fn on_run(&'_ mut self, world: &World) {
        self.0.on_run(world);
    }

    fn on_maintain(&'_ mut self, app_world: &mut World) {
        if self.1 {
            app_world.delete_all();
            self.0.runtime().create_engine::<Call>(app_world, "demo".to_string());
            self.1 = false;
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .compact()
        .init();

    for (index, arg) in args.iter().enumerate() {
        println!("add arg{} {}", index, arg);
    }

    println!("{:?}", args);
    if let Some(arg) = args.get(1) {
        println!("{}", arg);

        // starts without the UI
        if arg == "--headless" {
            main_headless();
        }

        // TODO add file_path
    }

    open("Lifec Demo Viewer", Demo::default(), Demo::new());
}

/// Example headless function, that reads a runmd file, creates a new engine
/// and then starts the engine. This allows the same file to be used with the UI and also w/o.
pub fn main_headless() {
    println!("Starting in headless mode, loading from .runmd");
    if let Some(file) = AttributeGraph::load_from_file(".runmd") {
       start(Demo::load(file), &["demo"]);
    }
}

impl AsRef<Runtime> for Demo {
    fn as_ref(&self) -> &Runtime {
        self.0.runtime()
    }
}

impl App for Demo {
    fn name() -> &'static str {
        "demo"
    }

    fn edit_ui(&mut self, _: &imgui::Ui) {}

    fn display_ui(&self, _: &imgui::Ui) {}
}

impl<'a> System<'a> for Demo {
    type SystemData = ();

    fn run(&mut self, _: Self::SystemData) {
    }
}
