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

        if ui.button("read project") {
            self.0.runtime_mut().read_project(app_world);
        }

        if ui.button("create sequence") {
            let runtime = self.0.runtime_mut();
            let mut sequence = Sequence::default();

            let config = lifec::plugins::Config("default_timer", |context|{
                context.block.block_name =  unique_title( "default_timer");
                context.as_mut().with_int("duration", 0).with_float_range("duration_ms", &[16.0, 0.0, 1000.0]);
            });

            // create by defining constants, statics
            if let Some(event) = runtime.create_event::<Call, Timer>(app_world, "timer_1") {
                sequence.add(event);
            }

            // create by predefining config
            if let Some(event) = runtime.create_with(app_world, &Call::event::<Timer>(), config) {
                sequence.add(event);
            }

            // create by predefining config
            if let Some(event) = runtime.create_with_name(app_world, &Call::event::<Timer>(), "timer_1") {
                sequence.add(event);
            }

            // create adhoc
            if let Some(event) = runtime.create(app_world, &Call::event::<Timer>(), |config| {
                config.block.block_name = "timer3".to_string();
                config.as_mut()
                    .with_int("duration", 3);
            }) {
                sequence.add(event);
            }

            if let Some(first) = sequence.next() {
                sequence.set_cursor(first);
                app_world.write_component::<Sequence>().insert(first, sequence).ok();
            }

            let mut sequence = Sequence::default();
            if let Some(event) = runtime.create_with_name(app_world, &Call::event::<Timer>(), "timer_1") {
                sequence.add(event);
                sequence.set_cursor(event);
                app_world.write_component::<Sequence>().insert(event, sequence).ok();
            }
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
        let mut demo = Demo::default();
        *demo.0.project_mut().as_mut() = file;
        let demo_project = demo.0.project_mut().reload_source();
        *demo.0.project_mut() = demo_project;

        demo.0.runtime_mut().add_config(Config("timer_1", |tc| {
            tc.block.block_name = unique_title("timer_1");
            tc.as_mut().with_int_range("duration", &[2, 0, 10]);
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

    fn display_ui(&self, ui: &imgui::Ui) {
    }
}

impl<'a> System<'a> for Demo {
    type SystemData = (
    );

    fn run(&mut self, data: Self::SystemData) {
   
    }
}