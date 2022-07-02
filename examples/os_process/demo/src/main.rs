use std::env;

use lifec::{editor::*, plugins::*, AttributeGraph};

#[derive(Default)]
struct Demo(RuntimeEditor, bool);

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
    }

    fn on_window_event(&'_ mut self, app_world: &World, event: &'_ WindowEvent<'_>) {
        match event {
            WindowEvent::DroppedFile(path) => {
                if "runmd" == path.extension().unwrap_or_default() {
                    if let Some(file) =
                        AttributeGraph::load_from_file(path.to_str().unwrap_or_default())
                    {
                        println!("{:#?}", file);
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
            self.0.runtime().create_engine::<Call>(app_world, "demo");
            self.1 = false;
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);
    if let Some(arg) = args.get(1) {
        println!("{}", arg);

        if arg == "--run_now" {
            main_headless();
        }

        // TODO add file_path
    }

    let mut demo = Demo::default();

    demo.0
        .runtime_mut()
        .add_config(Config("timer_simple", |tc| {
            tc.block.block_name = unique_title("simple");
            tc.as_mut().with_int_range("duration", &[1, 0, 10]);
        }));

    demo.0
        .runtime_mut()
        .add_config(Config("timer_complex", |tc| {
            tc.block.block_name = unique_title("complex");
            tc.as_mut().with_int_range("duration", &[1, 0, 10]);
            tc.as_mut()
                .with_float_range("duration_ms", &[0.0, 0.0, 1000.0]);
        }));

    demo.0.runtime_mut().add_config(Config("cargo_run", |tc| {
        tc.block.block_name = unique_title("cargo_run");
        tc.as_mut().with_text("command", "cargo run .");
    }));

    open("Lifec Demo Viewer", Demo::default(), demo);
}

/// Example headless function, that reads a runmd file, creates a new engine
/// and then starts the engine. This allows the same file to be used with the UI and also w/o.
pub fn main_headless() {
    if let Some(file) = AttributeGraph::load_from_file(".runmd") {
        let mut demo = Demo::default();
        *demo.0.project_mut().as_mut() = file;
        let demo_project = demo.0.project_mut().reload_source();
        *demo.0.project_mut() = demo_project;

        demo.0
            .runtime_mut()
            .add_config(Config("timer_simple", |tc| {
                tc.block.block_name = unique_title("simple");
                tc.as_mut().with_int_range("duration", &[1, 0, 10]);
            }));

        demo.0
            .runtime_mut()
            .add_config(Config("timer_complex", |tc| {
                tc.block.block_name = unique_title("complex");
                tc.as_mut().with_int_range("duration", &[1, 0, 10]);
                tc.as_mut()
                    .with_float_range("duration_ms", &[0.0, 0.0, 1000.0]);
            }));

        demo.0.runtime_mut().add_config(Config("cargo_run", |tc| {
            tc.block.block_name = unique_title("cargo_run");
            tc.as_mut().with_text("command", "cargo run .");
        }));

        let mut world = World::new();
        let mut dipatch_builder = DispatcherBuilder::new();
        Demo::configure_app_world(&mut world);
        Demo::configure_app_systems(&mut dipatch_builder);

        let mut dispatcher = dipatch_builder.build();
        dispatcher.setup(&mut world);

        if let Some(start) = demo.0.runtime().create_engine::<Call>(&world, "demo") {
            println!("Created engine {:?}", start);

            let mut event = world.write_component::<Event>();
            let tc = world.read_component::<ThunkContext>();
            let event = event.get_mut(start);
            if let Some(event) = event {
                if let Some(tc) = tc.get(start) {
                    event.fire(tc.clone());
                }
            }
        }

        loop {
            demo.on_run(&world);
            //demo.on_window_event(app_world, event)
            dispatcher.dispatch(&world);
            world.maintain();
            demo.on_maintain(&mut world);
        }
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

    fn run(&mut self, _: Self::SystemData) {}
}
