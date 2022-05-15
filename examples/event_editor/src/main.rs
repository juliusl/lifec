use std::fmt::Display;

use lifec::{
    editor::{start_editor, RuntimeEditor, Section, App},
    RuntimeState,
};
use specs::{Builder, WorldExt, System, WriteStorage, Join, Entities};
use specs::{Component, DenseVecStorage};

fn main() {
    start_editor(
        "Test Editor",
        1280.0,
        720.0,
         RuntimeEditor::<Test>::default(),
        |_, world, dispatcher| {
            world.register::<Section<Test>>();

            world
                .create_entity()
                .maybe_with(Some(Section::from(Test { id: 0, val: 10, clock: 0 })))
                .build();

            world
                .create_entity()
                .maybe_with(Some(Section::from(Test { id: 1, val: 11 , clock: 0})))
                .build();

            world
                .create_entity()
                .maybe_with(Some(Section::from(Test { id: 2, val: 12, clock: 0 })))
                .build();
            
            dispatcher.add(Incrementer{}, "random-system", &[]);
        },
    )
}

struct Incrementer;

impl <'a> System<'a> for Incrementer {
    type SystemData = (Entities<'a>, WriteStorage<'a, Section<Test>>);

    fn run(&mut self, mut data: Self::SystemData) {
        for e in data.0.join() {
           if let Some(section) = data.1.get_mut(e) {
               section.state.clock += 1;
           }
        }
    }
}

#[derive(Default, Clone)]
struct Test {
    val: i32,
    clock: i32,
    id: u32,
}

pub struct TestRuntimeError {}

impl App for Test {
    fn title() -> &'static str {
        "Test"
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        ui.input_int(format!("test_val: {}", self.id), &mut self.val)
            .build();
        ui.label_text(format!("count: {}", self.id), format!("{}", self.clock));
    }
}

impl Display for Test {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Test 1: {}", self.val)
    }
}

impl RuntimeState for Test {
    type Error = TestRuntimeError;

    fn load<S: AsRef<str> + ?Sized>(&self, _: &S) -> Self
    where
        Self: Sized,
    {
        self.clone()
    }

    fn process<S: AsRef<str> + ?Sized>(&self, _: &S) -> Result<Self, Self::Error> {
        Ok(self.clone())
    }
}

impl Component for Test {
    type Storage = DenseVecStorage<Self>;
}
