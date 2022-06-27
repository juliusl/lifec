use imgui::Window;
use lifec::{plugins::*, editor::*, AttributeGraph, Runtime};

fn main() {
    if let Some(file) = AttributeGraph::load_from_file("test_demo.runmd") {
        open(
            "demo",
            Runtime::new(Project::from(file)),
            Demo::default(),
        );
    }
}

#[derive(Default)]
struct Demo;

impl Extension for Demo {
    fn configure_app_world(world: &mut World) {
        Task::configure_app_world(world);

        let mut initial_context = ThunkContext::default();
        initial_context.as_mut().add_int_attr("duration", 5);

        let entity = Start::init(
            world
                .create_entity()
                .with(Start::event::<Timer>())
                .with(Task::default()),
        )
        .build();

        initial_context.entity = Some(entity);

        match world.write_component::<ThunkContext>().insert(entity, initial_context) {
            Ok(_) => {},
            Err(_) => {},
        }

        let mut initial_context = ThunkContext::default();
        initial_context.as_mut().add_text_attr("command", "cargo help");

        let entity = Start::init(
            world
                .create_entity()
                .with(Start::event::<Process>())
                .with(Task::default()),
        )
        .build();

        initial_context.entity = Some(entity);

        match world.write_component::<ThunkContext>().insert(entity, initial_context) {
            Ok(_) => {},
            Err(_) => {},
        }
    }

    fn configure_app_systems(dispatcher: &mut DispatcherBuilder) {
        Task::configure_app_systems(dispatcher);
    }

    fn on_ui(&'_ mut self, app_world: &World, ui: &'_ imgui::Ui<'_>) {
        Window::new("Demo").size([800.0, 600.0], imgui::Condition::Appearing).build(ui, ||{
            List::<Task>::default().on_ui(app_world, ui);
        });
    }

    fn on_window_event(&'_ mut self, _: &World, _: &'_ WindowEvent<'_>) {
    }

    fn on_run(&'_ mut self, _: &World) {
    }
}

