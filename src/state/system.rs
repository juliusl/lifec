use std::fmt::Display;

use atlier::system::{Attribute, Value};
use logos::{Lexer, Logos};
use specs::prelude::*;
use specs::storage::DenseVecStorage;
use specs::{Component, Entities, SystemData, World, WriteStorage};

use crate::{AttributeGraph, Runtime, RuntimeDispatcher, RuntimeState};

#[derive(Debug, Default, Clone)]
pub struct AttributeSystem;

impl Display for AttributeSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<AttributeGraph> for AttributeSystem {
    fn from(_g: AttributeGraph) -> Self {
        Self::default()
    }
}

impl RuntimeState for AttributeSystem {
    type State = AttributeGraph;
}

impl<'a> System<'a> for AttributeSystem {
    type SystemData = AttributeStore<'a>;

    fn setup(&mut self, world: &mut World) {
        AttributeStore::setup_world(world);
    }

    fn run(&mut self, mut data: Self::SystemData) {
        if let Some(mut runtime) = data.dispatch_runtime(self) {
            if runtime.can_continue() {
                let next = runtime.step();
                if let Some(_next) = next.current() {}
            }
        }
    }
}

#[derive(SystemData)]
pub struct AttributeStore<'a> {
    entities: Entities<'a>,
    expressions: WriteStorage<'a, AttributeExpression>,
    attributes: WriteStorage<'a, Attribute>,
    attributes_state: WriteStorage<'a, AttributeState>,
    graph: Write<'a, AttributeGraph>,
}

impl<'a> AsMut<AttributeGraph> for AttributeStore<'a> {
    fn as_mut(&mut self) -> &mut AttributeGraph {
        &mut self.graph
    }
}

impl<'a> AsRef<AttributeGraph> for AttributeStore<'a> {
    fn as_ref(&self) -> &AttributeGraph {
        &self.graph
    }
}

impl<'a> AttributeStore<'a> {
    pub fn remove_attribute(&mut self, entity_id: u32) {
        let edited = self.entities.entity(entity_id);

        self.expressions.remove(edited);
        self.attributes.remove(edited);
        self.attributes_state.remove(edited);
    }

    pub fn save_edited_attribute(&mut self, entity_id: u32) {
        let edited = self.entities.entity(entity_id);
        if let (Some(edited), Some(state)) = (
            self.attributes.get_mut(edited),
            self.attributes_state.get_mut(edited),
        ) {
            *edited = Self::save_edited(edited);
            *state = AttributeState::Saved;
        }
    }

    pub fn edit_reset_attribute(&mut self, entity_id: u32) {
        let editing = self.entities.entity(entity_id);
        if let (Some(editing), Some(state)) = (
            self.attributes.get_mut(editing),
            self.attributes_state.get_mut(editing),
        ) {
            Self::reset_editing(editing);
            *state = AttributeState::Transient;
        }
    }

    pub fn edit_commit_attribute(&mut self, entity_id: u32) {
        let editing = self.entities.entity(entity_id);
        if let (Some(editing), Some(state)) = (
            self.attributes.get_mut(editing),
            self.attributes_state.get_mut(editing),
        ) {
            *editing = Self::commit_editing(editing);
            *state = AttributeState::Edited;
        }
    }

    pub fn edit_attribute(&mut self, entity_id: u32) {
        let editing = self.entities.entity(entity_id);
        if let Some(editing) = self.attributes.get(editing) {
            let editing = editing.clone();
            if editing.is_stable() {
                let next = self.entities.create();
                match self.attributes.insert(
                    next,
                    Self::editing(next.id(), editing.name(), editing.value()),
                ) {
                    Ok(_) => {
                        if let None = self
                            .attributes_state
                            .insert(next, AttributeState::Transient)
                            .ok()
                        {
                            panic!(
                                "could not commit stable attribute state for entity {:?}",
                                next
                            );
                        }
                    }
                    Err(err) => {
                        eprintln!("Error: {}", err);
                        if let None = self
                            .attributes_state
                            .insert(next, AttributeState::Error)
                            .ok()
                        {
                            panic!(
                                "could not commit error attribute state for entity {:?}",
                                next
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn load_attribute(&mut self, entity_id: u32) {
        let loading = self.entities.entity(entity_id);
        if let Some(loading) = self.attributes.get(loading) {
            let loading = loading.clone();
            if loading.is_stable() {
                let next = self.entities.create();
                match self.attributes.insert(
                    next,
                    Self::stable(next.id(), loading.name(), loading.value()),
                ) {
                    Ok(_) => {
                        if let None = self
                            .attributes_state
                            .insert(next, AttributeState::Stable)
                            .ok()
                        {
                            panic!(
                                "could not commit stable attribute state for entity {:?}",
                                next
                            );
                        }
                    }
                    Err(err) => {
                        eprintln!("Error: {}", err);
                        if let None = self
                            .attributes_state
                            .insert(next, AttributeState::Error)
                            .ok()
                        {
                            panic!(
                                "could not commit error attribute state for entity {:?}",
                                next
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn new_attribute(&mut self, entity: Entity, attr_name: impl AsRef<str>) {
        let new_attribute = Self::new(entity.id(), attr_name);
        if let None = self.attributes.get(entity) {
            match self.attributes.insert(entity, new_attribute) {
                Ok(_) => {
                    if let None = self
                        .attributes_state
                        .insert(entity, AttributeState::Stable)
                        .ok()
                    {
                        panic!(
                            "could not commit stable attribute state for entity {:?}",
                            entity
                        );
                    }
                }
                Err(err) => {
                    eprintln!("Error: {}", err);
                    if let None = self
                        .attributes_state
                        .insert(entity, AttributeState::Error)
                        .ok()
                    {
                        panic!(
                            "could not commit error attribute state for entity {:?}",
                            entity
                        );
                    }
                }
            }
        }
    }

    fn new(id: u32, name: impl AsRef<str>) -> Attribute {
        Attribute::new(id, name.as_ref().to_string(), Value::Empty)
    }

    fn stable(id: u32, name: impl AsRef<str>, value: &Value) -> Attribute {
        Attribute::new(id, name.as_ref().to_string(), value.clone())
    }

    fn editing(id: u32, name: impl AsRef<str>, value: &Value) -> Attribute {
        let mut stable = Self::stable(id, name, value);
        stable.edit_self();
        stable
    }

    fn reset_editing(editing: &mut Attribute) -> &mut Attribute {
        editing.reset_editing();
        editing
    }

    fn commit_editing(editing: &mut Attribute) -> Attribute {
        editing.commit();
        let mut edited = Attribute::new(
            editing.id(),
            format!("edited::{}", editing.name()),
            Self::symbol_attribute_edited(),
        );
        edited.edit(editing.into());
        edited
    }

    fn save_edited(editing: &mut Attribute) -> Attribute {
        editing.commit();
        let mut edited = Attribute::new(
            editing.id(),
            format!("saved::{}", editing.name()),
            Self::symbol_attribute_saved(),
        );

        edited.edit(editing.into());
        edited
    }

    fn symbol_attribute_edited() -> Value {
        Value::Symbol("attribute_edited".to_string())
    }

    fn symbol_attribute_saved() -> Value {
        Value::Symbol("attribute_saved".to_string())
    }
}

/// defines an event that dispatches a message derived from `symbol` for `entity` 
fn define_for<S: RuntimeState>(entity: Entity, runtime: &mut Runtime<S>, symbol: impl AsRef<str>) {
    runtime
        .on(format!("{{ {};; {}_id }}", symbol.as_ref(), entity.id()))
        .dispatch(
            &format!("{} {}", symbol.as_ref(), entity.id()),
            &format!("{{ after_{};; {}_id }}", symbol.as_ref(), entity.id()),
        );
}

impl<'a> RuntimeDispatcher for AttributeStore<'a> {
    type Error = ();

    fn dispatch_runtime<S>(&mut self, current: &mut S) -> Option<Runtime<S>>
    where
        S: RuntimeState,
    {
        let mut runtime = Runtime::<S>::default();

        runtime.state = Some(current.clone());

        for entity in self.entities.join() {
            // define events active attributes
            if self.attributes.contains(entity) {
                define_for(entity, &mut runtime, "load");
                define_for(entity, &mut runtime, "edit");
                define_for(entity, &mut runtime, "edit_reset");
                define_for(entity, &mut runtime, "edit_commit");
                define_for(entity, &mut runtime, "save");
                define_for(entity, &mut runtime, "remove");
            }
        }

        Some(runtime)
    }

    fn dispatch_mut(&mut self, msg: impl AsRef<str>) -> Result<(), Self::Error> {
        let AttributeStore { entities, .. } = self;

        let mut lexer = AttributeEvents::lexer(msg.as_ref());
        match lexer.next() {
            Some(event) => match event {
                AttributeEvents::New(attr_name) => {
                    let entity = entities.create();

                    self.new_attribute(entity, attr_name);
                }
                AttributeEvents::Load(entity_id) => self.load_attribute(entity_id),
                AttributeEvents::Edit(entity_id) => self.edit_attribute(entity_id),
                AttributeEvents::EditReset(entity_id) => self.edit_reset_attribute(entity_id),
                AttributeEvents::EditCommit(entity_id) => self.edit_commit_attribute(entity_id),
                AttributeEvents::Save(entity_id) => self.save_edited_attribute(entity_id),
                AttributeEvents::Remove(entity_id) => self.remove_attribute(entity_id),
                AttributeEvents::Error => {}
            },
            None => {}
        };
        Ok(())
    }
}

#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct AttributeExpression {
    pub expression: String,
}

impl From<&str> for AttributeExpression {
    fn from(expression: &str) -> Self {
        AttributeExpression {
            expression: expression.to_string(),
        }
    }
}

#[derive(Logos, Debug, Hash, Clone, PartialEq, PartialOrd)]
pub enum AttributeEvents {
    /// Usage: new <attribute name>
    #[token("new", from_new_event)]
    New(String),
    /// Usage: load u32
    #[token("load", from_attribute_event)]
    Load(u32),
    /// Usage: edit u32
    #[token("edit", from_attribute_event)]
    Edit(u32),
    /// Usage: edit_reset u32
    #[token("edit_reset", from_attribute_event)]
    EditReset(u32),
    /// Usage: edit_commit u32
    #[token("edit_commit", from_attribute_event)]
    EditCommit(u32),
    /// Usage: save u32
    #[token("save", from_attribute_event)]
    Save(u32),
    /// Usage: remove u32
    #[token("remove", from_attribute_event)]
    Remove(u32),
    // Logos requires one token variant to handle errors,
    // it can be named anything you wish.
    #[error]
    // We can also use this variant to define whitespace,
    // or any other matches we wish to skip.
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

fn from_new_event(lex: &mut Lexer<AttributeEvents>) -> Option<String> {
    let attr_name = lex.remainder().trim().to_lowercase();
    Some(attr_name)
}

fn from_attribute_event(lex: &mut Lexer<AttributeEvents>) -> Option<u32> {
    lex.remainder().trim().parse().ok()
}

#[test]
fn test_attribute_events() {
    let mut test_lex = AttributeEvents::lexer("new test_attribute");
    assert_eq!(
        Some(AttributeEvents::New("test_attribute".to_string())),
        test_lex.next()
    );

    let mut test_lex = AttributeEvents::lexer("load 1");
    assert_eq!(Some(AttributeEvents::Load(1)), test_lex.next());

    let mut test_lex = AttributeEvents::lexer("edit 300");
    assert_eq!(Some(AttributeEvents::Edit(300)), test_lex.next());

    let mut test_lex = AttributeEvents::lexer("edit_reset 278");
    assert_eq!(Some(AttributeEvents::EditReset(278)), test_lex.next());

    let mut test_lex = AttributeEvents::lexer("edit_commit 279");
    assert_eq!(Some(AttributeEvents::EditCommit(279)), test_lex.next());

    let mut test_lex = AttributeEvents::lexer("save 179");
    assert_eq!(Some(AttributeEvents::Save(179)), test_lex.next());
}

#[derive(Component, Logos, Debug, Hash, Clone, PartialEq, PartialOrd)]
#[storage(DenseVecStorage)]
pub enum AttributeState {
    #[token("attribute_stable")]
    Stable,
    #[token("attribute_transient")]
    Transient,
    #[token("attribute_edited")]
    Edited,
    #[token("attribute_saved")]
    Saved,
    // Logos requires one token variant to handle errors,
    // it can be named anything you wish.
    #[error]
    // We can also use this variant to define whitespace,
    // or any other matches we wish to skip.
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}
