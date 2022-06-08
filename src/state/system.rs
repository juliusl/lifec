use std::fmt::Display;

use atlier::system::{Attribute, Value};
use logos::Logos;
use specs::prelude::*;
use specs::storage::DenseVecStorage;
use specs::{Component, Entities, Join, ReadStorage, System, SystemData, World, WriteStorage};

use crate::{AttributeGraph, RuntimeState, Runtime};

pub struct AttributeSystem {
    runtime: Option<Runtime::<AttributeSystemData<'static>>>
}

impl<'a> AttributeSystemData<'a> {
    pub fn new(id: u32, name: impl AsRef<str>) -> Attribute {
        Attribute::new(id, name.as_ref().to_string(), Value::Empty)
    }

    pub fn stable(id: u32, name: impl AsRef<str>, value: &Value) -> Attribute {
        Attribute::new(id, name.as_ref().to_string(), value.clone())
    }

    pub fn editing(id: u32, name: impl AsRef<str>, value: &Value) -> Attribute {
        let mut stable = Self::stable(id, name, value);
        stable.edit_self();
        stable
    }

    pub fn reset_editing(editing: &mut Attribute) -> &mut Attribute {
        editing.reset_editing();
        editing
    }

    pub fn commit_editing(editing: &mut Attribute) -> Attribute {
        editing.commit();
        let mut edited = Attribute::new(
            editing.id(),
            format!("edited::{}", editing.name()),
            Self::symbol_attribute_edited(),
        );
        edited.edit(editing.into());
        edited
    }

    pub fn save_edited(editing: &mut Attribute) -> Attribute {
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

#[derive(SystemData)]
pub struct AttributeSystemData<'a> {
    entities: Entities<'a>,
    expressions: ReadStorage<'a, AttributeExpression>,
    attributes: WriteStorage<'a, Attribute>,
    attributes_state: WriteStorage<'a, AttributeState>,
}

impl RuntimeState for AttributeSystemData<'static> {
    type Error = ();
    type State = AttributeGraph;

    fn dispatch_mut(&mut self, msg: impl AsRef<str>) -> Result<(), Self::Error> {
        let AttributeSystemData {
            entities,
            expressions,
            attributes,
            attributes_state,
        } = self;

        let mut lexer = AttributeEvents::lexer(msg.as_ref());
        match lexer.next() {
            Some(event) => {
                match event {
                    AttributeEvents::New => {
                        let attr_name = lexer.remainder().to_string();
                        let entity = entities.create();

                        let new_attribute = Self::new(entity.id(), attr_name);
                        if let None = attributes.get(entity) {
                            match attributes.insert(entity, new_attribute) {
                                Ok(_) => {
                                    if let None =
                                        attributes_state.insert(entity, AttributeState::Stable).ok()
                                    {
                                        panic!("could not commit stable attribute state for entity {:?}", entity);
                                    }
                                }
                                Err(err) => {
                                    eprintln!("Error: {}", err);
                                    if let None =
                                        attributes_state.insert(entity, AttributeState::Error).ok()
                                    {
                                        panic!("could not commit error attribute state for entity {:?}", entity);
                                    }
                                }
                            }
                        }
                    }
                    AttributeEvents::Load => match lexer.remainder().parse::<u32>() {
                        Ok(entity_id) => {
                            let loading = entities.entity(entity_id);
                            if let Some(loading) = attributes.get(loading) {
                                let loading = loading.clone();
                                if loading.is_stable() {
                                    let next = entities.create();
                                    match attributes.insert(
                                        next,
                                        Self::stable(next.id(), loading.name(), loading.value()),
                                    ) {
                                        Ok(_) => {
                                            if let None = attributes_state
                                                .insert(next, AttributeState::Stable)
                                                .ok()
                                            {
                                                panic!("could not commit stable attribute state for entity {:?}", next);
                                            }
                                        }
                                        Err(err) => {
                                            eprintln!("Error: {}", err);
                                            if let None = attributes_state
                                                .insert(next, AttributeState::Error)
                                                .ok()
                                            {
                                                panic!("could not commit error attribute state for entity {:?}", next);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => todo!(),
                    },
                    AttributeEvents::Edit => todo!(),
                    AttributeEvents::EditReset => todo!(),
                    AttributeEvents::EditCommit => todo!(),
                    AttributeEvents::Save => todo!(),
                    AttributeEvents::Error => todo!(),
                }
            }
            None => todo!(),
        }
        Ok(())
       
    }
}

impl System<'static> for AttributeSystem {
    type SystemData = AttributeSystemData<'static>;

    fn setup(&mut self, _world: &mut World) {
        let runtime = Runtime::<Self::SystemData>::default();

        self.runtime = Some(runtime); 
    }

    fn run(&mut self, state: Self::SystemData) {
       if let Some(runtime) = self.runtime.as_mut() {
            runtime.state = Some(state); 

            *runtime = runtime.step();
       }
    }
}

impl<'a> Clone for AttributeSystemData<'a> {
    fn clone(&self) -> Self {
        todo!("not implemented")
    }
}

impl<'a> Default for AttributeSystemData<'a> {
    fn default() -> Self {
        todo!("not implemented")
    }
}

impl<'a> From<AttributeGraph> for AttributeSystemData<'a> {
    fn from(_: AttributeGraph) -> Self {
        todo!("not implemented")
    }
}

impl<'a> Display for AttributeSystemData<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct AttributeExpression {
    pub expression: String,
}

#[derive(Logos, Debug, Hash, Clone, PartialEq, PartialOrd)]
pub enum AttributeEvents {
    /// Usage: new <attribute name>
    #[token("new")]
    New,
    /// Usage: load <u32>
    #[token("load")]
    Load,
    /// Usage: edit <u32>
    #[token("edit")]
    Edit,
    /// Usage: edit_reset <u32>
    #[token("edit_reset")]
    EditReset,
    /// Usage: edit_commit <u32>
    #[token("edit_commit")]
    EditCommit,
    /// Usage: save <u32>
    #[token("save")]
    Save,
    // Logos requires one token variant to handle errors,
    // it can be named anything you wish.
    #[error]
    // We can also use this variant to define whitespace,
    // or any other matches we wish to skip.
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
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
