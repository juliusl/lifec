use imgui::{Condition, Window};
use lifec::{editor::*, RuntimeState};
use ron::ser::PrettyConfig;
use specs::{Component, DenseVecStorage, RunNow};
use specs::{Entities, Join, ReadStorage, System, WorldExt, WriteStorage};
use std::fmt::Display;

fn main() {
    let mut node_editor = NodeEditor::new();
    let mut attribute_editor = AttributeEditor::new();

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
                    Clock::show_extension(s, ui);
                    if ui.button("hello") {
                        println!("world");
                    }

                    s.show_debug("test", ui);
                    s.show_debug("test-bool", ui);
                    s.show_debug("enable clock", ui);
                    s.show_debug("node::test int", ui);
                    s.show_debug("node::test float range", ui);
                    s.show_debug("binary", ui);
                    s.edit_attr("edit test attribute", "test", ui);
                    s.edit_attr("open a new window and test this attribute", "test-bool", ui);
                    s.edit_attr("enable clock for this section", "enable clock", ui);
                    s.edit_attr(
                        "enable node editor for this section",
                        "enable node editor",
                        ui,
                    );
                    s.edit_attr(
                        "edit a float that's from the node editor",
                        "node::test float",
                        ui,
                    );
                    s.edit_attr(
                        "edit a slider float",
                        "node::test float range",
                        ui
                    );
                    s.edit_attr(
                        "edit a float pair",
                        "node::test float pair",
                        ui
                    );

                    s.edit_attr_custom("node::test int pair", |attr| { 
                        ui.text(format!("testing edit attr custom {:?}", attr));
                    });

                    if let Some(true) = s.is_attr_checkbox("test-bool") {
                        Window::new("testing attr control")
                            .size([800.0, 600.0], Condition::Appearing)
                            .build(ui, || ui.text("hi"));
                    }

                    TestExtension::show_extension(s, ui);
                },
                Test {
                    id: 1,
                    val: 11,
                    clock: 1000,
                    open_test_window: false,
                },
            )
            .with_text("test", "hello")
            .with_bool("test-bool", false)
            .with_bool("enable clock", false)
            .with_bool("enable node editor", false)
            .with_bool("allow node editor to change state on close", false)
            .with_int("node::test int", 0)
            .with_int_pair("node::test int pair", &[0, 1])
            .with_float("node::test float", 0.0)
            .with_float_pair("node::test float pair", &[0.0, 1.0])
            .with_float_range("node::test float range", &[1.0, 2.0, 3.0])
            .with_attribute(Attribute::new(0, "binary".to_string(), Value::BinaryVector(vec![1, 2, 3])))
            .with_attribute(Attribute::new(0, "ref::test".to_string(), Value::TextBuffer("hello".to_string()).to_ref()))
            .with_file("test.txt")
            .with_file("doesnt_exist.txt")
            .enable_app_systems(),
        ],
        |w| {
            w.register::<Attribute>();
            NodeEditor::configure_app_world(w);
            AttributeEditor::configure_app_world(w);
        },
        |d| {
            d.add(Clock {}, "clock", &[]);
        },
        move |world, ui| {
            let node_editor = &mut node_editor;
            node_editor.extend_app_world(world, ui);

            let attribute_editor = &mut attribute_editor;
            attribute_editor.extend_app_world(world, ui);

            // example of extending the world with a custom window and save button
            Window::new("save the world")
                .size([800.0, 600.0], Condition::Appearing)
                .build(ui, || {
                    if ui.button("save") {
                        TestSerializeAttributes {}.run_now(world);
                    }
                });
        },
    );
}

struct TestSerializeAttributes;

impl<'a> System<'a> for TestSerializeAttributes {
    type SystemData = (Entities<'a>, ReadStorage<'a, SectionAttributes>);

    fn run(&mut self, (entities, section): Self::SystemData) {
        for entity in entities.join() {
            if let Some(section) = section.get(entity) {
                if let Some(str) = serde_json::to_string(section).ok() {
                    println!("{}", str);
                }

                if let Some(str) = ron::ser::to_string_pretty(section, PrettyConfig::new()).ok() {
                    println!("{}", str);
                }

                if let Some(vec) = rmp_serde::encode::to_vec(section).ok() {
                    println!("{:?}", vec);
                }
            }
        }
    }
}

struct TestExtension;

impl SectionExtension<Test> for TestExtension {
    fn show_extension(section: &mut Section<Test>, ui: &imgui::Ui) {
        ui.text(format!(
            "from test extension for {}, parent_entity: {}",
            section,
            section.get_parent_entity()
        ));
    }
}

struct Clock;

impl SectionExtension<Test> for Clock {
    fn show_extension(section: &mut Section<Test>, ui: &imgui::Ui) {
        if let Some(true) = section.is_attr_checkbox("enable clock") {
            ui.text(format!("clock: {}", section.state.clock));
        }
    }
}

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
