use std::{collections::BTreeMap, fmt::Display};

use atlier::system::{App, Attribute, Value};

use crate::{
    editor::{unique_title, Section, SectionAttributes},
    RuntimeState,
};

/// This trait is to organize different types of thunks
pub trait Thunk {
    fn symbol() -> &'static str; 

    fn call_with_context(context: &mut ThunkContext);

    fn call(values: &mut BTreeMap<String, Value>) {
        let mut context = ThunkContext::new("", Self::symbol(), values.clone());

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
}

impl ThunkContext {
    pub fn new(node_title: impl AsRef<str>, symbol: impl AsRef<str>, values: BTreeMap<String, Value>) -> Self {
        let node_title = node_title.as_ref().to_string();
        let symbol = symbol.as_ref().to_string();
        Self { node_title, symbol, values }
    }

    pub fn values_mut(&mut self) -> &mut BTreeMap<String, Value> {
       &mut self.values
    }

    pub fn set_output(&mut self, output: Value) {
        let symbol = self.symbol.clone();
        let values = self.values_mut();
        
        values.insert(format!("thunk::{}::output::", symbol), output);
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

    fn from_attributes(attributes: Vec<Attribute>) -> Self {
        let mut context = ThunkContext::default();

        if let Some(Value::Symbol(symbol)) = SectionAttributes::from(attributes.clone()).get_attr_value("symbol::") {
            context.symbol = symbol.to_string();
        } else {
            context.symbol = unique_title("anonymous");
        }

        attributes.iter().cloned().for_each(|a| {
            context
                .values
                .insert(a.name().to_string(), a.value().clone());
        });

        context
    }

    fn into_attributes(&self) -> Vec<Attribute> {
        let mut attributes = SectionAttributes::default();

        self.values.iter().clone().for_each(|(n, v)| {
            attributes.add_attribute(Attribute::new(0, n.strip_prefix("node::").unwrap_or(n), v.clone()));
        });

        attributes
            .with_attribute(Attribute::new(
                0,
                "symbol::",
                Value::Symbol(self.symbol.to_string()),
            ))
            .clone_attrs()
    }

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
            |s, ui| {
                s.state.into_attributes().iter().filter(|a| !a.name().starts_with("opened::")).for_each(|a| {
                    s.edit_attr(format!("edit {}", a.name()), a.name(), ui);
                });

                s.state = ThunkContext::from_attributes(s.attributes.iter().map(|(_, a)| a.clone()).collect());
            }, 
            self.clone());
        
        section.show_editor(ui);

        *self = section.state;
    }
}

#[test]
fn test_runtime_state() {
    let state = ThunkContext::from_attributes(
        SectionAttributes::default()
            .with_attribute(Attribute::new(
                0,
                "symbol::",
                Value::Symbol("thunk::test::".to_string()),
            ))
            .clone_attrs(),
    );

    assert_eq!(state.symbol, "thunk::test::");
}
