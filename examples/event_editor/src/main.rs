use std::fmt::Display;

use imgui::Window;
use lifec::{
    editor::{unique_title, App, Section, open_editor, Attribute, SectionAttributes},
    RuntimeState,
};
use specs::{Entities, Join, System, WriteStorage, WorldExt, ReadStorage};
use specs::{Component, DenseVecStorage};

fn main() {
    open_editor(vec![
        Section::from(Test {
            id: 0,
            val: 10,
            clock: 0,
            open_test_window: false,
        }).update(|s| s.add_int_attr("test", 10)),
        Section::new(
            unique_title("With Additional Tooling"),
            |s, ui| {
                Test::show_editor(&mut s.state, ui);

                ui.text(format!("clock: {}", s.state.clock));
                if ui.button("hello") {
                    println!("world");
                }

                s.show_attr_debug("display debug for test", "test", ui);
                s.show_attr_debug("display debug for test-bool", "test-bool", ui);
                s.edit_attr("edit test attribute", "test", ui);
                s.edit_attr("test checkbox", "test-bool", ui);
            },
            Test {
                id: 1,
                val: 11,
                clock: 0,
                open_test_window: false,
            },
        )
        .with_text("test", "hello")
        .with_bool("test-bool", false)
        .enable_app_systems(),
    ],
    |w| {
        w.register::<Attribute>();
    },
    |d| {
        d.add(Clock{}, "clock", &[]);
    });
}

struct Clock;

impl<'a> System<'a> for Clock {
    type SystemData = (Entities<'a>, WriteStorage<'a, Section<Test>>, ReadStorage<'a, SectionAttributes>);

    fn run(&mut self, mut data: Self::SystemData) {
        for e in data.0.join() {
            if let Some(section) = data.1.get_mut(e) {
                section.state.clock += 1;
            }

            if let Some(attributes) = data.2.get(e) {
                if let Some(attribute) = attributes.get_attr("test") {
                    println!("From Clock System, reading attribute: {:?}", attribute);
                }
            }
        }
    }
}

#[derive(Default, Clone, Component)]
#[storage(DenseVecStorage)]
struct Test {
    val: i32,
    id: u32,
    clock: u64,
    open_test_window: bool,
}

pub struct TestRuntimeError {}

impl App for Test {
    fn name() -> &'static str {
        "Test"
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        ui.input_int(format!("test_val: {}", self.id), &mut self.val)
            .build();
        if ui.checkbox(
            format!("test opening nested window {}", self.id),
            &mut self.open_test_window,
        ) {}

        if self.open_test_window {
            Window::new(format!("test window {}", self.id))
                .size([1280.0, 720.0], imgui::Condition::Appearing)
                .build(ui, || {
                    ui.text(format!("{}", self.clock));
                });
        }
    }
}

impl Display for Test {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
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

    fn merge_with(&self, other: &Self) -> Self {
        let mut next = self.clone();
        next.clock = other.clock;
        next
    }
}
