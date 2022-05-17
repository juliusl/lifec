use std::fmt::Display;

use imgui::{Window, Condition};
use lifec::{
    editor::*,
    RuntimeState,
};
use specs::{Component, DenseVecStorage};
use specs::{Entities, Join, ReadStorage, System, WorldExt, WriteStorage};

fn main() {
    open_editor(
        vec![
            Section::from(Test {
                id: 0,
                val: 10,
                clock: 0,
                open_test_window: false,
            })
            .with_title("Default Tooling")
            .with_int("test", 10),
            Section::new(
                unique_title("With Additional Tooling"),
                |s, ui| {
                    Test::show_editor(&mut s.state, ui);
                    ui.new_line();
                    ui.text(format!("clock: {}", s.state.clock));
                    if ui.button("hello") {
                        println!("world");
                    }

                    s.show_attr_debug("display debug for test", "test", ui);
                    s.show_attr_debug("display debug for test-bool", "test-bool", ui);
                    s.show_attr_debug("display debug for enable clock", "enable clock", ui);
                    s.edit_attr("edit test attribute", "test", ui);
                    s.edit_attr("open a new window and test this attribute", "test-bool", ui);
                    s.edit_attr("enable clock for this section", "enable clock", ui);

                    if let Some(true) = s.is_attr_checkbox("test-bool") {
                        Window::new("testing attr control").size([800.0, 600.0], Condition::Appearing).build(ui, || ui.text("hi"));
                    }

                    TestExtension::extend_section(s, ui);
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
            .with_bool("enable clock", false)
            .enable_app_systems(),
        ],
        |w| {
            w.register::<Attribute>();
        },
        |d| {
            d.add(Clock {}, "clock", &[]);
        },
    );
}

struct TestExtension;

impl SectionExtension<Test> for TestExtension {
    fn extend_section(section: &mut Section<Test>, ui: &imgui::Ui) {
        ui.text(format!("from test extension for {}, parent_entity: {}", section, section.get_parent_entity()));
    }
}

struct Clock;

impl<'a> System<'a> for Clock {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Section<Test>>,
        ReadStorage<'a, SectionAttributes>,
    );

    fn run(&mut self, mut data: Self::SystemData) {
        for e in data.0.join() {
            if let Some(attributes) = data.2.get(e) {
                if let Some(true) = attributes.is_attr_checkbox("enable clock") {
                    if let Some(section) = data.1.get_mut(e) {
                        section.state.clock += 1;
                    }
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
