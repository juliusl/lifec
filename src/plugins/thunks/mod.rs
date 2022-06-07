use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
};

use atlier::system::{App, Value};

use crate::{
    editor::{unique_title, Section},
    AttributeGraph, RuntimeState,
};

/// This trait is to organize different types of thunks
pub trait Thunk {
    fn symbol() -> &'static str;

    fn call_with_context(context: &mut ThunkContext);

    fn call(values: &mut AttributeGraph) {
        let mut context = ThunkContext::from(values.clone());

        Self::call_with_context(&mut context);

        *values = context.attribute_graph().clone();
    }
}

/// Thunk Context exists so that the contracts used in implementation
/// can depend on built in collection types, such as BTreeMap/Vec, etc
/// and Thunk Context can encapsulate methods to work with these collections
/// for common operations, such as setting output, and reading input
#[derive(Default, Clone)]
pub struct ThunkContext(AttributeGraph);

impl ThunkContext {
    pub fn new(
        node_title: impl AsRef<str>,
        symbol: impl AsRef<str>,
        values: BTreeMap<String, Value>,
    ) -> Self {
        Self({
            let mut attribute_graph = AttributeGraph::default()
                .with_text("node_title", node_title)
                .with_symbol("symbol", symbol);

            for (name, value) in values {
                attribute_graph.with(name, value);
            }

            attribute_graph
        })
    }

    pub fn node_title(&self) -> &String {
        if let Value::TextBuffer(node_title) = self
            .attribute_graph()
            .find_attr_value("node_title")
            .expect("thunk context has to be created with a node_title")
        {
            node_title
        } else {
            unreachable!("node_title is not Value::TextBuffer")
        }
    }

    pub fn symbol(&self) -> &String {
        if let Value::Symbol(symbol) = self
            .attribute_graph()
            .find_attr_value("symbol")
            .expect("thunk context has to be created with a symbol")
        {
            symbol
        } else {
            unreachable!("symbol is not a Value::Symbol")
        }
    }

    pub fn returns_key(&self) -> String {
        let symbol = self.symbol().clone();

        format!("{}::returns::", symbol)
    }

    pub fn outputs_key(&self, output_name: impl AsRef<str>) -> String {
        let symbol = self.symbol().clone();

        format!("{}::output::{}", symbol, output_name.as_ref())
    }

    pub fn find_value(&self, key: impl AsRef<str>) -> Option<&Value> {
        self.attribute_graph().find_attr_value(key)
    }

    pub fn set_returns(&mut self, returns: Value) {
        let returns_key = &self.returns_key();

        self.attribute_graph_mut()
            .with(returns_key.to_string(), returns.clone());
    }

    pub fn returns(&self) -> Option<&Value> {
        let returns_key = &self.returns_key();

        self.attribute_graph().find_attr_value(returns_key)
    }

    pub fn set_output(&mut self, output_key: impl AsRef<str>, output: Value) {
        let output_key = self.outputs_key(output_key);

        self.attribute_graph_mut().with(output_key, output);
    }

    pub fn get_outputs(&self) -> Vec<(String, &Value)> {
        let symbol = self.symbol().clone();
        let prefix = format!("{}::output::", symbol);

        self.attribute_graph()
            .iter_attributes()
            .filter(|a| a.name().starts_with(&prefix))
            .map(|a| (a.name().to_string(), a.value()))
            .collect()
    }
}

#[test]
fn test_thunk_context() {
    let mut test_values = BTreeMap::default();

    test_values.insert("test_value".to_string(), Value::Int(10));

    let thunk_context = ThunkContext::new("test", "test", test_values);

    assert_eq!(
        thunk_context.find_value("test_value"),
        Some(&Value::Int(10))
    );
    assert_eq!(thunk_context.node_title(), "test");
    assert_eq!(thunk_context.symbol(), "test");
}

impl Display for ThunkContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

pub struct ThunkError;

impl From<AttributeGraph> for ThunkContext {
    fn from(attribute_graph: AttributeGraph) -> Self {
        if attribute_graph.contains_attribute("node_title")
            && attribute_graph.contains_attribute("symbol")
        {
            Self(attribute_graph)
        } else {
            Self(
                attribute_graph
                    .clone()
                    .with_text("node_title", unique_title("node"))
                    .with_symbol("symbol", unique_title("anonymous"))
            )
        }
    }
}

impl RuntimeState for ThunkContext {
    type Error = ThunkError;

    /// process
    fn process(&self, _: impl AsRef<str>) -> Result<Self, Self::Error> {
        todo!("not implemented")
    }

    fn attribute_graph(&self) -> &AttributeGraph {
        &self.0
    }

    fn attribute_graph_mut(&mut self) -> &mut AttributeGraph {
        &mut self.0
    }
}

impl App for ThunkContext {
    fn name() -> &'static str {
        "Thunk Context"
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        let mut section = Section::<ThunkContext>::new(
            format!("{} {}", self.symbol(), self.node_title()),
            AttributeGraph::default(),
            |s, ui| {
                let mut set = BTreeSet::new();

                let mut attributes = s.state.attribute_graph().clone();
                attributes
                    .iter_mut_attributes()
                    .filter(|a| {
                        !a.name().starts_with("opened::") && !a.name().starts_with("symbol::")
                    })
                    .for_each(|a| {
                        set.insert(a);
                    });

                for a in set {
                    s.edit_attr(format!("{} [{}]", a.name(), s.title), a.name(), ui);
                }

                s.state = ThunkContext::from(attributes);
            },
            self.clone(),
        );

        section.show_editor(ui);

        *self = section.state;
    }
}

#[test]
fn test_runtime_state() {
    let state = ThunkContext::from(
        AttributeGraph::default().with("symbol::", Value::Symbol("thunk::test::".to_string())),
    );

    assert_eq!(state.symbol(), "thunk::test::");
}
