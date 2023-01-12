use atlier::system::Extension;
use reality::{Attribute, AttributeParser, SpecialAttribute};
use serde::{Deserialize, Serialize};
use specs::{storage::HashMapStorage, Component, WorldExt};

use crate::{engine::Adhoc, state::AttributeGraph};

/// Pointer struct for implementing a Form Component
///
/// # Background
///
/// Sometimes it is useful to reuse operations in a way that allows developers to input config settings on demand.
/// The form feature is a special attribute that can point to a operation and apply control values on demand.
///
/// For example given an operation,
///
/// ```norun
/// + .operation speak
/// : .println {greeting} {note}
/// : .fmt greeting, note
/// ```
///
/// It would be useful to edit greeting in the editor tool, especially if the operation will be used multiple times,
///
/// In the root, a form would be defined like,
///
/// ```norun
/// + .form         speak
/// : .description  Enter a greeting that will be printed
/// : .require      greeting .symbol Hello World
/// : .optional     note     .symbol
/// ```
///
/// This defines a form, that will have an input for the symbol property "greeting" and "note".
///
/// When rendered, the editor will lookup the operation "speak" and compile it and then open a window prompt with an input
/// widget to specify the value of greeting.
///
///
#[derive(Component, Serialize, Deserialize)]
#[storage(HashMapStorage)]
pub struct Form {
    adhoc: Adhoc,
    elements: Vec<FormElement>,
}

/// Enumeration of elements in the form,
///
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormElement {
    Description(String),
    Required(Attribute),
    Optional(Attribute),
}

impl Form {
    /// Returns the adhoc definition this form references,
    ///
    pub fn adhoc(&self) -> &Adhoc {
        &self.adhoc
    }

    /// Returns a mutable iterator over form elements,
    ///
    pub fn iter_elements_mut(&mut self) -> impl Iterator<Item = &mut FormElement> {
        self.elements.iter_mut()
    }

    /// Returns an immutable iterator over form elements,
    ///
    pub fn iter_elements(&self) -> impl Iterator<Item = &FormElement> {
        self.elements.iter()
    }
}

impl SpecialAttribute for Form {
    fn ident() -> &'static str {
        "form"
    }

    fn parse(parser: &mut AttributeParser, content: impl AsRef<str>) {
        let world = parser.world().expect("should have a world").clone();

        let form_entity = world.entities().create();
        parser.set_id(form_entity.id());

        parser.add_custom_with("description", |p, c| {
            if let Some(entity) = p.entity() {
                let description = c;
                p.lazy_exec_mut(move |world| {
                    if let Some(form) = world.write_component::<Form>().get_mut(entity) {
                        form.elements.push(FormElement::Description(description));
                    }
                });
            }
        });

        parser.add_custom_with("require", |p, c| {
            if let Some(entity) = p.entity() {
                if let Some(attribute) = AttributeParser::default().parse(c).next() {
                    p.lazy_exec_mut(move |world| {
                        if let Some(form) = world.write_component::<Form>().get_mut(entity) {
                            form.elements.push(FormElement::Required(attribute));
                        }
                    });
                }
            }
        });

        parser.add_custom_with("optional", |p, c| {
            if let Some(entity) = p.entity() {
                if let Some(attribute) = AttributeParser::default().parse(c).next() {
                    p.lazy_exec_mut(move |world| {
                        if let Some(form) = world.write_component::<Form>().get_mut(entity) {
                            form.elements.push(FormElement::Optional(attribute));
                        }
                    });
                }
            }
        });

        if let Ok(adhoc) = Adhoc::from_parser(content, "form", parser) {
            world
                .write_component()
                .insert(
                    form_entity,
                    Form {
                        adhoc,
                        elements: vec![],
                    },
                )
                .expect("should be able to insert");
        }
    }
}

impl Extension for Form {
    fn on_ui(&'_ mut self, _: &specs::World, ui: &'_ imgui::Ui<'_>) {
        self.render_elements(ui);
    }
}

impl Form {
    /// Renders input widgets for elements of the form,
    ///
    fn render_elements(&mut self, ui: &imgui::Ui) {
        /// Renders input for editing an attribute
        /// 
        fn edit_attribute(idx: usize, attr: &mut Attribute, ui: &imgui::Ui) {
            let name = &attr.name;

            AttributeGraph::edit_value(format!("{name}##{idx}"), attr.value_mut(), None, ui);
        }

        for (idx, element) in self.iter_elements_mut().enumerate() {
            match element {
                FormElement::Description(description) => {
                    ui.text(description);
                }
                FormElement::Required(attr) => {
                    edit_attribute(idx, attr, ui);
                }
                FormElement::Optional(attr) => {
                    edit_attribute(idx, attr, ui);
                }
            }
        }
    }
}

mod tests {
    /// Tests the result of parsing a form attribute
    ///
    #[test]
    fn test_attribute_parse() {
        use super::Form;
        use crate::editor::form::FormElement;
        use crate::engine::Adhoc;
        use reality::{Attribute, Parser, Value};
        use specs::{World, WorldExt};

        let mut world = World::new();
        world.register::<Form>();

        let parser = Parser::new_with(world).with_special_attr::<Form>().parse(
            r#"
                ```
                # Testing w/o tag
                + .form speak
                : .description  Test description
                : .require      greeting    .symbol
                : .optional     note        .symbol

                # Testing w/ tag
                + test .form speak
                : .description  Test description
                : .require      greeting    .symbol Hello Test
                : .optional     note        .symbol
                ```
            "#,
        );

        let mut world = parser.commit();
        world.maintain();

        let form_entity = world.entities().entity(1);
        let form = world
            .write_component::<Form>()
            .remove(form_entity)
            .expect("should have a form");
        assert_eq!(
            form.elements
                .get(0)
                .expect("should have a form element at this position"),
            &FormElement::Description(String::from("Test description"))
        );
        assert_eq!(
            form.elements
                .get(1)
                .expect("should have a form element at this position"),
            &FormElement::Required(Attribute::new(
                0,
                "greeting",
                Value::Symbol(String::default())
            ))
        );
        assert_eq!(
            form.elements
                .get(2)
                .expect("should have a form element at this position"),
            &FormElement::Optional(Attribute::new(0, "note", Value::Symbol(String::default())))
        );
        assert_eq!(
            form.adhoc,
            Adhoc {
                name: String::from("speak"),
                tag: String::from("operation")
            }
        );

        let form_entity = world.entities().entity(2);
        let form = world
            .write_component::<Form>()
            .remove(form_entity)
            .expect("should have a form");
        assert_eq!(
            form.elements
                .get(0)
                .expect("should have a form element at this position"),
            &FormElement::Description(String::from("Test description"))
        );
        assert_eq!(
            form.elements
                .get(1)
                .expect("should have a form element at this position"),
            &FormElement::Required(Attribute::new(
                0,
                "greeting",
                Value::Symbol(String::from("Hello Test"))
            ))
        );
        assert_eq!(
            form.elements
                .get(2)
                .expect("should have a form element at this position"),
            &FormElement::Optional(Attribute::new(0, "note", Value::Symbol(String::default())))
        );
        assert_eq!(
            form.adhoc,
            Adhoc {
                name: String::from("speak"),
                tag: String::from("test.operation")
            }
        );

        // Testing w/o registering attribute
        // Assert that no entities will be created as a result of parsing unregistered form attribute
        let parser = Parser::new().parse(
            r#"
            ```
            # Testing w/o tag
            + .form speak
            : .description  Test description
            : .require      greeting    .symbol
            : .optional     note        .symbol

            # Testing w/ tag
            + test .form speak
            : .description  Test description
            : .require      greeting    .symbol Hello Test
            : .optional     note        .symbol
            ```
        "#,
        );

        let mut world = parser.commit();
        world.maintain();
        assert_eq!(world.entities().create().id(), 1);
    }
}
