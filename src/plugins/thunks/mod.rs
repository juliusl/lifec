use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
};

use atlier::system::{App, Value};

use crate::{
    editor::Section,
    RuntimeState, 
    AttributeGraph,
};

/// This trait is to organize different types of thunks
pub trait Thunk {
    fn symbol() -> &'static str;

    fn call_with_context(context: &mut ThunkContext);

    fn call(values: &mut BTreeMap<String, Value>) {
        let mut context =
            ThunkContext::new("", format!("thunk::{}", Self::symbol()), values.clone());

        Self::call_with_context(&mut context);

        *values = context.values_mut().clone();
    }
}

/// Thunk Context exists so that the contracts used in implementation
/// can depend on built in collection types, such as BTreeMap/Vec, etc
/// and Thunk Context can encapsulate methods to work with these collections
/// for common operations, such as setting output, and reading input
#[derive(Default, Clone)]
pub struct ThunkContext {
    node_title: String,
    symbol: String,
    values: BTreeMap<String, Value>,
    _a: AttributeGraph
}

impl ThunkContext {
    pub fn new(
        node_title: impl AsRef<str>,
        symbol: impl AsRef<str>,
        values: BTreeMap<String, Value>,
    ) -> Self {
        let node_title = node_title.as_ref().to_string();
        let symbol = symbol.as_ref().to_string();
        Self {
            node_title,
            symbol,
            values,
            _a: AttributeGraph::default()
        }
    }

    pub fn returns_key(&self) -> String {
        let symbol = self.symbol.clone();

        format!("{}::returns::", symbol)
    }

    pub fn outputs_key(&self, output_name: impl AsRef<str>) -> String {
        let symbol = self.symbol.clone();

        format!("{}::output::{}", symbol, output_name.as_ref())
    }

    pub fn get_value(&self, key: impl AsRef<str>) -> Option<Value> {
        self.values.get(key.as_ref()).and_then(|v| Some(v.clone()))
    }

    pub fn values_mut(&mut self) -> &mut BTreeMap<String, Value> {
        &mut self.values
    }

    pub fn set_returns(&mut self, returns: Value) {
        let returns_key = &self.returns_key();
        let values = self.values_mut();

        values.insert(returns_key.to_string(), returns.clone());
    }

    pub fn returns(&self) -> Option<&Value> {
        let returns_key = &self.returns_key();

        self.values.get(returns_key)
    }

    pub fn set_output(&mut self, output_key: impl AsRef<str>, output: Value) {
        let output_key = self.outputs_key(output_key);
        let values = self.values_mut();

        values.insert(output_key, output);
    }

    pub fn get_outputs(&self) -> Vec<(&String, &Value)> {
        let symbol = self.symbol.clone();
        let prefix = format!("{}::output::", symbol);

        self.values
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .collect()
    }
}

impl Display for ThunkContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

pub struct ThunkError;

impl RuntimeState for ThunkContext {
    type Error = ThunkError;

    // fn from_attributes(attributes: Vec<Attribute>) -> Self {
    //     let mut context = ThunkContext::default();

    //     if let Some(Value::Symbol(symbol)) =
    //         SectionAttributes::from(attributes.clone()).get_attr_value("symbol::")
    //     {
    //         context.symbol = symbol.to_string();
    //     } else {
    //         context.symbol = unique_title("anonymous");
    //     }

    //     attributes.iter().cloned().for_each(|a| {
    //         context
    //             .values
    //             .insert(a.name().to_string(), a.value().clone());
    //     });

    //     context
    // }

    // fn into_attributes(&self) -> Vec<Attribute> {
    //     let mut attributes = SectionAttributes::default();

    //     self.values.iter().clone().for_each(|(n, v)| {
    //         attributes.add_attribute(Attribute::new(
    //             0,
    //             n.strip_prefix("node::").unwrap_or(n),
    //             v.clone(),
    //         ));
    //     });

    //     attributes
    //         .with_attribute(Attribute::new(
    //             0,
    //             "symbol::",
    //             Value::Symbol(self.symbol.to_string()),
    //         ))
    //         .clone_attrs()
    // }

    /// process
    fn process<S: AsRef<str> + ?Sized>(&self, _: &S) -> Result<Self, Self::Error> {
        todo!("not implemented")
    }
}

impl App for ThunkContext {
    fn name() -> &'static str {
        "Thunk Context"
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        let mut section = Section::<ThunkContext>::new(
            format!("{} {}", self.symbol, self.node_title),
            AttributeGraph::default(),
            |s, ui| {
                let mut set = BTreeSet::new();

                let mut attributes = s.state.attribute_graph().clone();
                attributes.iter_mut_attributes()
                    .filter(|a| {
                        !a.name().starts_with("opened::") && !a.name().starts_with("symbol::")
                    })
                    .for_each(|a| {
                        set.insert(a);
                    });

                for a in set {
                    s.edit_attr(format!("{} [{}]", a.name(), s.title), a.name(), ui);
                }

                s.state = ThunkContext::from_attribute_graph(attributes);
            },
            self.clone(),
        );

        section.show_editor(ui);

        *self = section.state;
    }
}

#[test]
fn test_runtime_state() {
    let state = ThunkContext::from_attribute_graph(
        AttributeGraph::default()
            .with(
                "symbol::",
                Value::Symbol("thunk::test::".to_string()))
    );

    assert_eq!(state.symbol, "thunk::test::");
}
