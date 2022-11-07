use crate::prelude::*;
use atlier::system::{Attribute, Value};
use imgui::Ui;
use reality::BlockProperties;
use specs::Component;
use std::{
    collections::{hash_map::DefaultHasher, BTreeMap},
    hash::{Hash, Hasher},
};

/// Wrapper struct over a block index,
///
/// Implements AttributeIndex
///
#[derive(Debug, Default, Component, Clone, Hash, Eq, PartialEq, PartialOrd)]
#[storage(VecStorage)]
pub struct AttributeGraph {
    /// Block index,
    ///
    index: BlockIndex,
    /// Scopes the graph to a child entity,
    ///
    child: Option<u32>,
    /// Optional config that will be applied to this graph,
    ///
    applied: Vec<BlockProperties>,
}

impl AttributeGraph {
    /// Creates an attribute graph over data found in a block,
    ///
    pub fn new(index: BlockIndex) -> Self {
        Self {
            index,
            child: None,
            applied: vec![],
        }
    }

    /// Returns a reference to index,
    ///
    pub fn index(&self) -> &BlockIndex {
        &self.index
    }

    /// Returns a mutable reference to index,
    ///
    pub fn index_mut(&mut self) -> &mut BlockIndex {
        &mut self.index
    }

    /// Applies a config to the graph's control values,
    ///
    /// Stores the applied block properties,
    ///
    pub fn apply(&mut self, config: BlockProperties) {
        event!(Level::TRACE, "Applying config {:#?}", config);
        for (name, property) in config.iter_properties() {
            match property {
                BlockProperty::Single(value) => {
                    self.add_control(name, value.clone());
                }
                BlockProperty::List(values) => {
                    // Control values can only have a single value, so apply the last value in the list
                    // Since by default the indexer will convert a duplicate named attribute into a list property
                    let last = values.last().expect("should have a last value");
                    self.add_control(name, last.clone());
                }
                _ => {}
            }
        }

        self.applied.push(config);
    }

    /// Adds a control value to the underlying graph,
    ///
    /// A control value will be available to every plugin that consumes this graph.
    ///
    pub fn add_control(&mut self, name: impl AsRef<str>, value: impl Into<Value>) {
        self.index.add_control(name, value);
    }

    /// Returns the current hash_code of the graph
    ///
    pub fn hash_code(&self) -> u64 {
        let mut hasher = DefaultHasher::default();

        self.hash(&mut hasher);

        hasher.finish()
    }

    /// Returns some bool if there is a matching name attribute with bool value
    ///
    pub fn is_enabled(&self, with_name: impl AsRef<str>) -> bool {
        self.find_bool(with_name).unwrap_or_default()
    }

    /// Returns a new graph scoped at the child entity,
    ///
    /// If the child is not a part of this graph, nothing is returned
    ///
    pub fn scope(&self, child: u32) -> Option<AttributeGraph> {
        if let Some(_) = self.index.child_properties(child) {
            let mut clone = self.clone();
            clone.child = Some(child);
            Some(clone)
        } else {
            None
        }
    }

    /// Returns an unscoped graph,
    ///
    pub fn unscope(&self) -> AttributeGraph {
        let mut clone = self.clone();
        clone.child = None;
        clone
    }

    /// Resolves the properties to use within the current scope,
    ///
    pub fn resolve_properties(&self) -> &BlockProperties {
        if let Some(child) = self
            .child
            .and_then(|child| self.index.child_properties(child))
        {
            child
        } else {
            self.index.properties()
        }
    }

    /// Resolves the properties to use within the current scope,
    ///
    pub fn resolve_properties_mut(&mut self) -> &mut BlockProperties {
        if let Some(child) = self.child {
            self.index.child_properties_mut(child).unwrap()
        } else {
            self.index.properties_mut()
        }
    }
}

impl AttributeIndex for AttributeGraph {
    fn entity_id(&self) -> u32 {
        if let Some(child) = self.child {
            child
        } else {
            self.index.root().id()
        }
    }

    fn values(&self) -> BTreeMap<String, Vec<Value>> {
        let mut values = BTreeMap::default();
        for (name, property) in self.resolve_properties().iter_properties() {
            let mut property_values = vec![];

            match property {
                BlockProperty::Single(val) => {
                    property_values.push(val.clone());
                }
                BlockProperty::List(vals) => {
                    let mut vals = vals.iter().cloned().collect();
                    let vals = &mut vals;
                    property_values.append(vals);
                }
                _ => {
                    continue;
                }
            }

            values.insert(name.to_string(), property_values);
        }

        values
    }

    fn hash_code(&self) -> u64 {
        self.hash_code()
    }

    fn find_value(&self, with_name: impl AsRef<str>) -> Option<Value> {
        let search = |property: Option<BlockProperty>| match property {
            Some(property) => match property {
                BlockProperty::Single(value) => Some(value),
                BlockProperty::List(values) => values.first().cloned(),
                BlockProperty::Required(_) => {
                    event!(
                        Level::ERROR,
                        "Required property has not been set, {}",
                        with_name.as_ref()
                    );
                    None
                }
                BlockProperty::Optional(_) => {
                    event!(
                        Level::WARN,
                        "Optional property has not been set, {}",
                        with_name.as_ref()
                    );
                    None
                }
                BlockProperty::Empty => None,
            },
            None => {
                event!(
                    Level::TRACE,
                    "Could not find any property {}",
                    with_name.as_ref()
                );
                None
            }
        };

        let properties = self.resolve_properties();
        match search(properties.property(with_name.as_ref()).cloned()) {
            Some(val) => Some(val),
            None => {
                event!(
                    Level::TRACE,
                    "Searching for `{}` from control values",
                    with_name.as_ref()
                );
                self.index.control_values().get(with_name.as_ref()).cloned()
            }
        }
    }

    fn find_values(&self, with_name: impl AsRef<str>) -> Vec<Value> {
        let search = |property: Option<BlockProperty>| match property {
            Some(property) => match property {
                BlockProperty::Single(value) => vec![value],
                BlockProperty::List(values) => values.clone(),
                BlockProperty::Required(_) => {
                    event!(
                        Level::ERROR,
                        "Required property has not been set, {}",
                        with_name.as_ref()
                    );
                    vec![]
                }
                BlockProperty::Optional(_) => {
                    event!(
                        Level::WARN,
                        "Optional property has not been set, {}",
                        with_name.as_ref()
                    );
                    vec![]
                }
                BlockProperty::Empty => {
                    vec![]
                }
            },
            None => {
                event!(
                    Level::TRACE,
                    "Could not find any property {}",
                    with_name.as_ref()
                );
                vec![]
            }
        };

        let properties = self.resolve_properties();
        let mut output = search(properties.property(with_name.as_ref()).cloned());

        if output.is_empty() {
            event!(
                Level::TRACE,
                "Searching for `{}` from control values",
                with_name.as_ref()
            );
            if let Some(val) = self.index.control_values().get(with_name.as_ref()) {
                output.push(val.clone());
            }
        }
        output
    }

    fn add_attribute(&mut self, attr: Attribute) {
        let root = self.index.root().name().to_string();

        let properties = if self.index.root().id() != attr.id() {
            self.index
                .child_properties_mut(attr.id)
                .expect("Trying to add an attribute that is out of context of the current index")
        } else {
            self.index.properties_mut()
        };

        if attr.is_stable() {
            // If added through this with/add functions, then the attribute should
            // always be stable
            properties.add(attr.name, attr.value.clone());
        } else if let Some((name, value)) = attr.transient {
            let name = name.trim_start_matches(&root);
            properties.add(name, value.clone());
        }
    }

    fn replace_attribute(&mut self, attr: Attribute) {
        let root = self.index.root().name().to_string();

        let properties = if self.index.root().id() != attr.id() {
            self.index
                .child_properties_mut(attr.id)
                .expect("Trying to add an attribute that is out of context of the current index")
        } else {
            self.index.properties_mut()
        };

        if attr.is_stable() {
            // If added through this with/add functions, then the attribute should
            // always be stable
            properties.set(attr.name, BlockProperty::Single(attr.value.clone()));
        } else if let Some((name, value)) = attr.transient {
            let name = name.trim_start_matches(&root);
            properties.set(name, BlockProperty::Single(value.clone()));
        }
    }

    fn properties(&self) -> &BlockProperties {
        self.resolve_properties()
    }

    fn properties_mut(&mut self) -> &mut BlockProperties {
        self.resolve_properties_mut()
    }

    fn control_values(&self) -> &BTreeMap<String, Value> {
        self.index.control_values()
    }
}

impl AttributeGraph {
    /// Edit value,
    ///
    pub fn edit_value(
        name: impl AsRef<str>,
        value: &mut Value,
        workspace: Option<Workspace>,
        ui: &Ui,
    ) {
        match value {
            atlier::system::Value::Empty => {
                ui.label_text(name, "empty");
            }
            atlier::system::Value::Bool(b) => {
                ui.checkbox(name, b);
            }
            atlier::system::Value::TextBuffer(text) => {
                ui.input_text_multiline(name, text, [140.0, 160.0]).build();
            }
            atlier::system::Value::Int(i) => {
                ui.input_int(name, i).build();
            }
            atlier::system::Value::IntPair(a, b) => {
                let clone = &mut [*a, *b];
                ui.input_int2(name, clone).build();
                *a = clone[0];
                *b = clone[1];
            }
            atlier::system::Value::IntRange(a, b, c) => {
                let clone = &mut [*a, *b, *c];
                ui.input_int3(name, clone).build();
                *a = clone[0];
                *b = clone[1];
                *c = clone[2];
            }
            atlier::system::Value::Float(f) => {
                ui.input_float(name, f).build();
            }
            atlier::system::Value::FloatPair(a, b) => {
                let clone = &mut [*a, *b];
                ui.input_float2(name, clone).build();
                *a = clone[0];
                *b = clone[1];
            }
            atlier::system::Value::FloatRange(a, b, c) => {
                let clone = &mut [*a, *b, *c];
                ui.input_float3(name, clone).build();
                *a = clone[0];
                *b = clone[1];
                *c = clone[2];
            }
            atlier::system::Value::BinaryVector(_) => {
                if let Some(_) = workspace {
                    if ui.button(format!("Save {} to file", name.as_ref())) {}
                }
            }
            atlier::system::Value::Reference(_) => {}
            atlier::system::Value::Symbol(s) => {
                // TODO: add debouncing?
                ui.input_text(name, s).build();
            }
            atlier::system::Value::Complex(_) => {}
        }
    }
}

impl App for AttributeGraph {
    fn name() -> &'static str {
        "graph"
    }

    fn edit_ui(&mut self, ui: &imgui::Ui) {
        let id = self.entity_id();

        for (name, property) in self.resolve_properties_mut().iter_properties_mut() {
            property.edit(
                move |value| Self::edit_value(format!("{name} {id}"), value, None, ui),
                move |values| {
                    imgui::ListBox::new(format!("{name} {id}")).build(ui, || {
                        for (idx, value) in values.iter_mut().enumerate() {
                            Self::edit_value(format!("{name} {id}-{idx}"), value, None, ui);
                        }
                    });
                },
                || None,
            )
        }
    }

    fn display_ui(&self, ui: &imgui::Ui) {
        for (name, property) in self.resolve_properties().iter_properties() {
            ui.text(format!("{name}: {property}"));
        }

        for (name, value) in self.index.control_values() {
            ui.text(format!("{name}: {:?}", value));
        }
    }
}
