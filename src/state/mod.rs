
use atlier::system::{Attribute, Value};
use imgui::{TableFlags, Ui};
use serde::{Deserialize, Serialize};
use specs::{storage::HashMapStorage, Component, Entity};
use tracing::{event, Level};
use std::{
    borrow::Cow,
    fmt::Display,
    str::from_utf8,
    hash::{Hash, Hasher},
    collections::{hash_map::DefaultHasher, BTreeMap},
};

mod v2;
pub use v2::AttributeIndex;
pub use v2::Query;
pub use v2::Operation;

/// Attribute graph is a component that indexes attributes for an entity
/// It is designed to be a general purpose enough to be the common element of runtime state storage
/// 
#[derive(Debug, Default, Component, Clone, Hash, Serialize, Deserialize, Eq, PartialEq, PartialOrd)]
#[storage(HashMapStorage)]
pub struct AttributeGraph {
    entity: u32,
    index: BTreeMap<String, Attribute>,
}

impl AttributeIndex for AttributeGraph {
    fn entity_id(&self) -> u32 {
        self.entity
    }

    fn hash_code(&self) -> u64 {
        self.hash_code()
    }

    fn find_value(&self, with_name: impl AsRef<str>) -> Option<Value> {
        self.find_attr_value(with_name).and_then(|v| Some(v.to_owned()))
    }

    fn find_transient(&self, with_name: impl AsRef<str>, with_symbol: impl AsRef<str>) -> Option<&Attribute> {
        let key = format!("{}::{}", with_name.as_ref(), with_symbol.as_ref());
        self.find_attr(key)
    }

    fn add_attribute(&mut self, attr: Attribute) {
        self.index.insert(attr.name.to_string(), attr);
    }

    fn define(&mut self, name: impl AsRef<str>, symbol: impl AsRef<str>) -> &mut Attribute {
        let name = format!("{}::{}", name.as_ref(), symbol.as_ref());
        self.index.insert(
            name.to_string(), 
            Attribute::new(self.entity, &name, Value::Empty)
        );
        self.index.get_mut(&name).expect("just added")
    }
}

impl AttributeGraph {
     /// Interpret several msgs w/ a clone of self
     pub fn batch(&self, msgs: impl AsRef<str>) -> Result<Self, AttributeGraphErrors>
     where
         Self: Clone,
     {
         let next = self.clone();
         for message in msgs
             .as_ref()
             .trim()
             .lines()
             .filter(|line| !line.trim().is_empty())
         {
             // next = next.dispatch(message)?;

             todo!()
         }
 
         Ok(next)
     }
 
     /// Interpret several msgs, applying changes to self
     pub fn batch_mut(&mut self, msg: impl AsRef<str>) -> Result<(), AttributeGraphErrors> {
         for message in msg
             .as_ref()
             .trim()
             .split("\n")
             .map(|line| line.trim())
             .filter(|line| !line.is_empty())
         {
             // self.dispatch_mut(message)?;

             todo!()
         }
         Ok(())
     }

    /// returns the graph after attributes are committed and processed
    pub fn commit(&self, process: impl FnOnce(&mut AttributeGraph)) -> AttributeGraph {
        let mut saving = self.clone();
        process(&mut saving);

        for attr in saving.clone().iter_attributes() {
            saving.find_update_attr(attr.name(), |a| {
                a.commit();
            });
        }
        saving
    }

    /// finds and applies events to the graph
    pub fn apply_events(&mut self) {
        self.apply("event");
    }

    /// add event which will be applied when commit is called
    pub fn add_event(&mut self, event_name: impl AsRef<str>, message: impl AsRef<str>) {
        self.add_message(event_name, "event", message)
    }

    /// add message to graph that can be dispatched with apply(..)
    pub fn add_message(
        &mut self,
        name: impl AsRef<str>,
        symbol: impl AsRef<str>,
        message: impl AsRef<str>,
    ) {
        self.define(name.as_ref(), symbol.as_ref())
            .edit_as(Value::BinaryVector(message.as_ref().as_bytes().to_vec()));
    }

    /// finds and applies all messages to graph
    pub fn apply(&mut self, symbol: impl AsRef<str>) {
        for (name, value) in self.take_symbol_values(symbol.as_ref()) {
            if let Value::BinaryVector(content) = value {
                if let Some(content) = from_utf8(&content).ok() {

                    match self.batch_mut(content) {
                        Ok(_) => {
                            self.find_remove(name);
                        }
                        Err(err) => {
                            event!(Level::ERROR, "could not apply events, {:?}", err);
                        }
                    }
                }
            }
        }
    }

    /// iterates through all missing values
    pub fn values_missing(&self) -> impl Iterator<Item = &Attribute> {
        self.iter_attributes()
            .filter(|a| a.id() == self.entity())
            .filter_map(|a| match a.value() {
                Value::Empty => Some(a),
                _ => None,
            })
            .into_iter()
    }

    /// returns true of all attributes are stable
    pub fn is_stable(&self) -> bool {
        self.iter_attributes()
            .filter(|a| a.id() == self.entity())
            .all(|a| a.is_stable())
    }

    /// loads an attribute graph from file
    pub fn load_from_file(path: impl AsRef<str>) -> Option<Self> {
        // let mut loading = AttributeGraph::default();

        // match loading.from_file(&path) {
        //     Ok(_) => {
        //         let loaded = loading.define("src", "file");
        //         loaded.edit_as(Value::TextBuffer(path.as_ref().to_string()));

        //         event!(Level::TRACE, "loading .runmd file {}", path.as_ref());
        //         Some(loading)
        //     }
        //     Err(err) => {
        //         event!(Level::ERROR, "Could not load {}, {:?}", path.as_ref(), err);
        //         None
        //     }
        // }
        todo!()
    }

    /// Returns the current hash_code of the graph
    pub fn hash_code(&self) -> u64 {
        let mut hasher = DefaultHasher::default();

        self.hash(&mut hasher);

        hasher.finish()
    }

    /// returns the owning entity
    pub fn entity(&self) -> u32 {
        self.entity
    }

    /// using self as source, include a sibling block given the context
    pub fn include_block(
        &self,
        mut context: impl AsRef<AttributeGraph> + AsMut<AttributeGraph>,
        symbol_name: impl AsRef<str>,
    ) {
        if let Some(block_name) = context.as_ref().find_text("block_name") {
            if let Some(form) = self.find_block(block_name, symbol_name) {
                context.as_mut().merge(&form);
            }
        }
    }

    /// if a block_name is set, finds the form block and displays an edit form
    pub fn edit_form_block(&self, ui: &imgui::Ui) -> Option<AttributeGraph> {
        if let Some(mut block) = self.find_block("", "form") {
            let hash_code = block.hash_code();
            for attr in block
                .iter_mut_attributes()
                .filter(|a| !a.name.starts_with("block_"))
            {
                match attr.value() {
                    Value::Symbol(_) => {}
                    _ => attr.edit_value("", ui),
                }
            }
            if hash_code != block.hash_code() {
                Some(block)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// This method allows you to edit an attribute from this section
    /// You can use a label that is different from the actual attribute name
    /// This allows attribute re-use
    pub fn edit_attr(
        &mut self,
        label: impl AsRef<str> + Display,
        attr_name: impl AsRef<str>,
        ui: &imgui::Ui,
    ) {
        if let Some(width) = self.find_float("edit_width") {
            ui.set_next_item_width(width);
        } else {
            ui.set_next_item_width(0.0);
        }

        let label = format!("{} {}", label, self.entity);
        let attr_name = attr_name.as_ref().to_string();

        match self.find_attr_mut(&attr_name) {
            Some(attr) => {
                attr.edit_value(label, ui);
            }
            None => {
                ui.text(format!("'{}' not found", &attr_name));
            }
        }
    }

    /// This method allows you to create a custom editor for your attribute,
    /// in case the built in methods are not enough
    pub fn edit_attr_custom(&mut self, attr_name: impl AsRef<str>, show: impl Fn(&mut Attribute)) {
        if let Some(attr) = self.find_attr_mut(attr_name) {
            show(attr);
        }
    }

    pub fn edit_attr_menu(&mut self, ui: &imgui::Ui) {
        if let Some(token) = ui.begin_menu("File") {
            let file_name = format!("{}.ron", self.hash_code());

            if imgui::MenuItem::new(format!("Save to {}", file_name)).build(ui) {
                // if fs::write(&file_name, self.save().unwrap_or_default()).is_ok() {
                //     println!("Saved output to {}", file_name);
                // }
                todo!()
            }

            if let Some(file_source) = self
                .clone()
                .find_attr("src::file")
                .and_then(|a| a.transient())
                .and_then(|(_, v)| {
                    if let Value::TextBuffer(file_source) = v {
                        Some(file_source)
                    } else {
                        None
                    }
                })
            {
                if imgui::MenuItem::new(format!("Reload source {}", &file_source)).build(ui) {
                    // if self.from_file(&file_source).is_ok() {
                    //     println!("Reloaded {}", &file_source);
                    // }
                        todo!()
                }
            }

            ui.separator();
            token.end()
        }
    }

    /// This method shows an attribute table
    pub fn edit_attr_table(&mut self, ui: &imgui::Ui) {
        if let Some(token) = ui.begin_table_with_flags(
            format!("Attribute Graph Table {}", self.entity),
            5,
            TableFlags::RESIZABLE | TableFlags::SORTABLE,
        ) {
            ui.table_setup_column("Name");
            ui.table_setup_column("Value");
            ui.table_setup_column("State");
            ui.table_setup_column("Reference");
            ui.table_setup_column("Key");
            ui.table_headers_row();

            let clone = self.clone();
            let mut attrs: Vec<&Attribute> = clone.iter_attributes().collect();

            if let Some(mut sorting) = ui.table_sort_specs_mut() {
                attrs.sort_by(|a, b| {
                    let mut order = a.cmp(b);
                    for spec in sorting.specs().iter() {
                        order = match spec.column_idx() {
                            0 => a.name().cmp(b.name()),
                            1 => a.value().cmp(b.value()),
                            2 => a.is_stable().cmp(&b.is_stable()),
                            3 => a.value().to_ref().cmp(&b.value().to_ref()),
                            4 => a.to_string().cmp(&b.to_string()),
                            _ => a.cmp(b),
                        };
                        if let Some(dir) = spec.sort_direction() {
                            match dir {
                                imgui::TableSortDirection::Descending => order = order.reverse(),
                                _ => {}
                            }
                        }
                    }
                    order
                });
                sorting.set_sorted();
            }

            for attr in attrs {
                if ui.table_next_column() {
                    ui.text(attr.name());
                }

                if ui.table_next_column() {
                    if attr.id() != self.entity() {
                        ui.text(format!("imported {}", attr.id()));
                    } else {
                        self.edit_attr(attr.name(), attr.name(), ui);
                    }
                }

                if ui.table_next_column() {
                    if attr.is_stable() {
                        ui.text("stable");
                    } else {
                        if attr
                            .transient()
                            .and_then(|(_, v)| if let Value::Empty = v { None } else { Some(()) })
                            .is_some()
                        {
                            ui.text("transient");
                        } else {
                            ui.text("defined");
                        }
                    }
                }

                if ui.table_next_column() {
                    ui.text(attr.value().to_ref().to_string());
                }

                if ui.table_next_column() {
                    ui.text(attr.to_string());
                }

                ui.table_next_row();
            }

            token.end();
        }
    }

    /// This method shows an attribute table
    pub fn edit_attr_short_table(&mut self, ui: &imgui::Ui) {
        if let Some(token) = ui.begin_table_with_flags(
            format!("Attribute Graph Table {}", self.entity),
            3,
            TableFlags::RESIZABLE | TableFlags::SORTABLE,
        ) {
            ui.table_setup_column("Name");
            ui.table_setup_column("Value");
            ui.table_setup_column("State");
            ui.table_headers_row();

            let clone = self.clone();
            let mut attrs: Vec<&Attribute> = clone.iter_attributes().collect();

            if let Some(mut sorting) = ui.table_sort_specs_mut() {
                attrs.sort_by(|a, b| {
                    let mut order = a.cmp(b);
                    for spec in sorting.specs().iter() {
                        order = match spec.column_idx() {
                            0 => a.name().cmp(b.name()),
                            1 => a.value().cmp(b.value()),
                            2 => a.is_stable().cmp(&b.is_stable()),
                            _ => a.cmp(b),
                        };
                        if let Some(dir) = spec.sort_direction() {
                            match dir {
                                imgui::TableSortDirection::Descending => order = order.reverse(),
                                _ => {}
                            }
                        }
                    }
                    order
                });
                sorting.set_sorted();
            }

            for attr in attrs {
                if ui.table_next_column() {
                    ui.text(attr.name());
                }

                if ui.table_next_column() {
                    if attr.id() != self.entity() {
                        ui.text(format!("imported {}", attr.id()));
                    } else {
                        self.edit_attr(attr.name(), attr.name(), ui);
                    }
                }

                if ui.table_next_column() {
                    if attr.is_stable() {
                        ui.text("stable");
                    } else {
                        if attr
                            .transient()
                            .and_then(|(_, v)| if let Value::Empty = v { None } else { Some(()) })
                            .is_some()
                        {
                            ui.text("transient");
                        } else {
                            ui.text("defined");
                        }
                    }
                }

                ui.table_next_row();
            }

            token.end();
        }
    }

    /// Copies all the values from other graph
    pub fn copy(&mut self, other: &AttributeGraph) {
        other.iter_attributes().for_each(|a| {
            self.copy_attribute(a);
        })
    }

    /// Imports all the values from other graph
    pub fn import(&mut self, other: &AttributeGraph) {
        other.iter_attributes().for_each(|a| {
            self.import_attribute(a);
        })
    }

    /// merge the values from the other graph
    pub fn merge(&mut self, other: &AttributeGraph) {
        for attr in other.iter_attributes().cloned() {
            if !self.index.contains_key(&attr.to_string()) {
                self.index.insert(attr.to_string(), attr.clone());
            } else {
                if other.entity != self.entity {
                    let name = &attr.name();
                    self.find_update_imported_attr(attr.id(), name, |existing| {
                        if existing.value() != attr.value() {
                            *existing.value_mut() = attr.value().clone();
                        }
                    });
                } else {
                    let name = &attr.name();
                    self.find_update_attr(name, |existing| {
                        if existing.value() != attr.value() {
                            *existing.value_mut() = attr.value().clone();
                        }
                    });
                }
            }
        }
    }

    /// Returns true if the graph has an attribute w/ name
    pub fn contains_attribute(&self, with_name: impl AsRef<str>) -> bool {
        self.find_attr(with_name).is_some()
    }

    /// Returns some bool if there is a matching name attribute with bool value.
    pub fn is_enabled(&self, with_name: impl AsRef<str>) -> Option<bool> {
        if let Some(Value::Bool(val)) = self.find_attr_value(with_name) {
            Some(*val)
        } else {
            None
        }
    }

    /// Returns some bool if an attribute with_name exists and is a symbol,
    /// true if the symbol value starts with_symbol
    pub fn is_defined(
        &self,
        with_name: impl AsRef<str>,
        with_symbol: impl AsRef<str>,
    ) -> Option<bool> {
        if let Some(Value::Symbol(val)) = self.find_attr_value(with_name) {
            Some(val.starts_with(with_symbol.as_ref()))
        } else {
            None
        }
    }

    /// Updates the parent entity id of the graph.
    pub fn set_parent_entity(&mut self, parent: Entity) {
        self.set_parent_entity_id(parent.id());
    }

    /// Sets the current parent entity id.
    /// The parent entity id is used when adding attributes to the graph.
    pub fn set_parent_entity_id(&mut self, entity_id: u32) {
        // Update only attributes that the current parent owns
        // attributes that have a different id are only in the collection as references
        let current = self.clone();
        let current_id = current.entity;

        current
            .iter_attributes()
            .filter(|a| a.id() == current_id)
            .for_each(|a| {
                self.find_update_attr(a.name(), |a| a.set_id(entity_id));
            });

        // Finally update the id
        self.entity = entity_id;
    }

    /// Import an attribute that can have a different entity id.
    /// If the external_attribute has the same id as parent entity, this will instead be a no-op.
    /// This behavior is to enforce that attributes should be added with the below api's.
    pub fn import_attribute(&mut self, external_attribute: &Attribute) {
        if external_attribute.id() == self.entity {
            event!(Level::WARN, "No-Op, Trying to import an attribute that is not external to this graph, add this attribute by value instead");
            return;
        }
        self.add_attribute(external_attribute.clone());
    }

    /// Copies an attribute and add's it as being owned by the parent entity.
    pub fn copy_attribute(&mut self, external_attribute: &Attribute) {
        let mut copy = external_attribute.clone();
        copy.set_id(self.entity);

        self.add_attribute(copy);
    }

    /// Finds and removes an attribute w/ name.
    pub fn find_remove(&mut self, with_name: impl AsRef<str>) -> Option<Attribute> {
        let finding = self.clone();
        let finding = finding.find_attr(with_name);
        if let Some(attr) = finding {
            self.remove(attr)
        } else {
            None
        }
    }

    /// Removes an attribute from the index, returns the removed attribute.
    pub fn remove(&mut self, attr: &Attribute) -> Option<Attribute> {
        self.index.remove(&attr.to_string())
    }

    /// Clears the attribute index.
    pub fn clear_index(&mut self) {
        self.index.clear();
    }

    /// Returns true if the index is empty.
    pub fn is_index_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Returns a mut iterator over indexed attributes.
    pub fn iter_mut_attributes(&mut self) -> impl Iterator<Item = &mut Attribute> {
        self.index.iter_mut().map(|(_, a)| a)
    }

    /// Returns an iterator over indexed attributes.
    pub fn iter_attributes(&self) -> impl Iterator<Item = &Attribute> {
        self.index.values().into_iter()
    }

    /// Finds the value of an attribute by name that is owned by `self.entity`.
    pub fn find_attr_value(&self, with_name: impl AsRef<str>) -> Option<&Value> {
        let name = with_name.as_ref().to_string();
        let key = format!("{:#010x}::{name}::", self.entity);
        if let Some(attr) = self.index.get(&key) {
            Some(&attr.value)
        } else {
            None
        }
    }

    /// Finds the mut value of an attribute by name that is owned by `self.entity`.
    pub fn find_attr_value_mut(&mut self, with_name: impl AsRef<str>) -> Option<&mut Value> {
        self.find_attr_mut(with_name)
            .and_then(|a| Some(a.value_mut()))
    }

    /// Finds an attribute by name that is owned by `self.entity`
    pub fn find_attr(&self, with_name: impl AsRef<str>) -> Option<&Attribute> {
        self.iter_attributes()
            .filter(|attr| attr.id() == self.entity)
            .find(|attr| attr.name() == with_name.as_ref())
            .and_then(|a| Some(a))
    }

    /// Finds a mut attribute by name that is owned by `self.entity`
    pub fn find_attr_mut(&mut self, with_name: impl AsRef<str>) -> Option<&mut Attribute> {
        let current_id = self.entity;
        self.iter_mut_attributes()
            .filter(|attr| attr.id() == current_id)
            .find(|attr| attr.name() == with_name.as_ref())
            .and_then(|a| Some(a))
    }

    /// Finds the parent block if the graph has been in block mode
    pub fn find_parent_block(&mut self) -> Option<&mut Attribute> {
        self.iter_mut_attributes()
            .find(|attr| attr.name() == "parent::block")
            .and_then(|a| Some(a))
    }

    /// Finds the parent block if the graph has been in block mode
    pub fn find_last_block(&mut self) -> Option<Attribute> {
        let clone = self.clone();
        let last_block = clone
            .iter_attributes()
            .find(|attr| attr.name() == "last::block")
            .and_then(|a| Some(a));

        if let Some(last_block) = last_block {
            self.remove(last_block)
        } else {
            None
        }
    }

    /// Finds and updates an attribute, also updates index key.
    /// Returns true if update was called.
    pub fn find_update_attr(
        &mut self,
        with_name: impl AsRef<str>,
        update: impl FnOnce(&mut Attribute),
    ) -> bool {
        if let Some(attr) = self.find_attr_mut(with_name) {
            let old_key = attr.to_string();
            update(attr);

            // it's possible that the name changed, remove/add the attribute to update the key
            if let Some(attr) = self.index.remove(&old_key) {
                self.add_attribute(attr);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Finds an attribute by symbol that is owned by `self.entity`
    pub fn find_symbols(&self, with_symbol: impl AsRef<str>) -> Vec<&Attribute> {
        let symbol = format!("{}::", with_symbol.as_ref());
        self.index
            .iter()
            .filter(|(_, a)| {
                if let Attribute {
                    id,
                    value: Value::Symbol(value),
                    ..
                } = a
                {
                    *id == self.entity && value.starts_with(&symbol)
                } else {
                    false
                }
            })
            .map(|(_, a)| a)
            .collect()
    }

    /// Finds a mut attribute by symbol that is owned by `self.entity`
    pub fn find_symbols_mut(&mut self, with_symbol: impl AsRef<str>) -> Vec<&mut Attribute> {
        let symbol = format!("{}::", with_symbol.as_ref());
        let current_id = self.entity;
        self.index
            .iter_mut()
            .filter(|(_, a)| {
                if let Attribute {
                    id,
                    value: Value::Symbol(value),
                    ..
                } = a
                {
                    *id == current_id && value.starts_with(&symbol)
                } else {
                    false
                }
            })
            .map(|(_, a)| a)
            .collect()
    }

    /// Returns a map of current symbol values, from symbol transients
    pub fn find_symbol_values(&self, with_symbol: impl AsRef<str>) -> Vec<(String, Value)> {
        self.find_symbols(with_symbol)
            .iter()
            .filter_map(|a| a.transient())
            .cloned()
            .collect()
    }

    /// Returns a vec of current symbol values, from symbol transients in the current root
    pub fn iter_transient_root_values(&self) -> Vec<(String, Value)> {
        self.index
            .iter()
            .filter(|(_, a)| {
                if let Attribute {
                    id,
                    value: Value::Symbol(_),
                    ..
                } = a
                {
                    *id == self.entity
                } else {
                    false
                }
            })
            .filter_map(|(_, a)| a.transient())
            .cloned()
            .collect()
    }

    /// Returns a vec of current symbol values, from symbol transients not owned by this root
    pub fn iter_transient_imported_values(&self) -> Vec<(String, Value)> {
        self.index
            .iter()
            .filter(|(_, a)| {
                if let Attribute {
                    id,
                    value: Value::Symbol(_),
                    ..
                } = a
                {
                    *id != self.entity
                } else {
                    false
                }
            })
            .filter_map(|(_, a)| a.transient())
            .cloned()
            .collect()
    }

    /// Returns a vec of current symbol values, from symbol transients not owned by this root
    pub fn iter_transient_values(&self) -> Vec<(String, Value)> {
        self.index
            .iter()
            .filter_map(|(_, a)| {
                if let Attribute {
                    value: Value::Symbol(_),
                    ..
                } = a
                {
                    a.transient()
                } else {
                    None
                }
            })
            .cloned()
            .collect()
    }

    /// Takes all transient values
    pub fn take_symbol_values(&mut self, with_symbol: impl AsRef<str>) -> Vec<(String, Value)> {
        self.find_symbols_mut(with_symbol)
            .iter_mut()
            .filter_map(|a| a.take_transient())
            .map(|a| a)
            .collect()
    }

    /// Finds a graph that has been imported by this graph.
    pub fn find_imported_graph(&self, id: u32) -> Option<Self> {
        let mut imported = Self::from(id);

        self.iter_attributes()
            .filter(|attr| attr.id() == id)
            .for_each(|a| {
                imported.copy_attribute(a);
            });

        if imported.is_index_empty() {
            None
        } else {
            Some(imported)
        }
    }

    /// find only imported symbols by name
    pub fn find_imported_symbols(&self, with_symbol: impl AsRef<str>) -> Vec<&Attribute> {
        let symbol = format!("{}::", with_symbol.as_ref());
        self.index
            .iter()
            .filter(|(_, a)| {
                if let Attribute {
                    id,
                    value: Value::Symbol(value),
                    ..
                } = a
                {
                    *id != self.entity && value.starts_with(&symbol)
                } else {
                    false
                }
            })
            .map(|(_, a)| a)
            .collect()
    }

    /// find only imported symbols with transient values
    pub fn find_imported_symbol_values(
        &self,
        with_symbol: impl AsRef<str>,
    ) -> Vec<(String, Value)> {
        self.find_imported_symbols(with_symbol)
            .iter()
            .filter_map(|a| a.transient())
            .cloned()
            .collect()
    }

    /// Finds and updates an attribute, also updates index key.
    /// Returns true if update was called.
    pub fn find_update_imported_attr(
        &mut self,
        with_id: u32,
        with_name: impl AsRef<str>,
        update: impl FnOnce(&mut Attribute),
    ) -> bool {
        if let Some(attr) = self.find_imported_attr(with_id, with_name) {
            let old_key = attr.to_string();
            update(attr);

            // it's possible that the name changed, remove/add the attribute to update the key
            if let Some(attr) = self.index.remove(&old_key) {
                self.add_attribute(attr);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Finds a mut attribute by name that is owned by `self.entity`
    pub fn find_imported_attr(
        &mut self,
        with_id: u32,
        with_name: impl AsRef<str>,
    ) -> Option<&mut Attribute> {
        self.iter_mut_attributes()
            .filter(|attr| attr.id() == with_id)
            .find(|attr| attr.name() == with_name.as_ref())
            .and_then(|a| Some(a))
    }

    /// find all blocks by symbol name
    pub fn find_blocks(&self, symbol_name: impl AsRef<str>) -> Vec<Self> {
        let mut clone = self.clone();
        clone.entity = 0; // set to 0 to bring it back up to the root

        clone
            .find_imported_symbol_values(symbol_name)
            .iter()
            .filter_map(|(_, value)| {
                if let Value::Int(block_id) = value {
                    self.find_imported_graph(*block_id as u32)
                } else {
                    None
                }
            })
            .collect()
    }

    /// finds the block_id for with the corresponding block name
    pub fn find_block_id(
        &self,
        with_name: impl AsRef<str>,
        symbol_name: impl AsRef<str>,
    ) -> Option<u32> {
        let symbol = format!("{}::", symbol_name.as_ref());
        let symbol_name = format!("{}::{}", with_name.as_ref(), symbol_name.as_ref());

        self.index
            .iter()
            .find_map(|(_, a)| {
                if let Attribute {
                    value: Value::Symbol(value),
                    transient: Some((name, Value::Int(block_id))),
                    ..
                } = a
                {
                    if value.starts_with(&symbol)
                        && value.ends_with("block")
                        && name == &symbol_name
                    {
                        Some(block_id)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .and_then(|id| Some(*id as u32))
    }

    /// finds the block for with the corresponding block name
    pub fn find_block(
        &self,
        with_name: impl AsRef<str>,
        symbol_name: impl AsRef<str>,
    ) -> Option<Self> {
        if with_name.as_ref().is_empty() {
            if let Some(block_name) = self.find_text("block_name") {
                self.find_block_id(block_name, symbol_name)
                    .and_then(|id| self.find_imported_graph(id))
            } else {
                None
            }
        } else {
            self.find_block_id(with_name, symbol_name)
                .and_then(|id| self.find_imported_graph(id))
        }
    }

    /// iterates each block in the graph
    pub fn iter_blocks(&self) -> impl Iterator<Item = Self> + '_ {
        self.index
            .iter()
            .filter_map(|(_, a)| {
                if let Attribute {
                    name,
                    value: Value::Symbol(maybe_block_symbol),
                    transient: Some((transient_name, Value::Int(_))),
                    ..
                } = a
                {
                    if name == transient_name && maybe_block_symbol.ends_with("::block") {
                        let parts: Vec<&str> = name.split("::").collect();
                        if let (Some(name), Some(symbol)) = (parts.get(0), parts.get(1)) {
                            self.find_block(name, symbol)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .into_iter()
    }

    pub fn find_blocks_for(&self, block_name: impl AsRef<str>) -> impl Iterator<Item = Self> + '_ {
        let block_name = block_name.as_ref().to_string();
        self.iter_blocks()
            .filter(move |b| {
                if let Some(filtered) = b
                    .find_text("block_name")
                    .and_then(|b| Some(b.as_str() == block_name))
                {
                    filtered
                } else {
                    false
                }
            })
            .into_iter()
    }

    pub fn from_block(
        &mut self,
        block_name: impl AsRef<str>,
        block_symbol: impl AsRef<str>,
        attr_name: impl AsRef<str>,
    ) {
        let mut root = self.clone();
        root.entity = 0;

        if let Some(block) = root.find_block(block_name, block_symbol) {
            if let Some(attr_value) = block.find_attr(&attr_name) {
                let (name, value) = (attr_value.name(), attr_value.value());
                self.with(name, value.clone());
            }
        }
    }

    pub fn to_block(&mut self, block_symbol: impl AsRef<str>, attr_name: impl AsRef<str>) {
        if let Some(block_name) = self.find_text("block_name") {
            if let Some(block) = &self.find_block(block_name, block_symbol) {
                if let Some(attr) = self.clone().find_attr(&attr_name) {
                    let current = self.entity;
                    self.entity = block.entity;
                    if self.find_update_attr(&attr_name, |a| {
                        a.edit_as(attr.value().clone());
                    }) {
                        self.define(attr_name, "link")
                            .edit_as(Value::IntPair(current as i32, block.entity as i32));
                    }
                    self.entity = current;
                }
            }
        }
    }
}

impl AttributeGraph {
    /// Displays a combo box over a set of symbols
    /// 
    /// The chosen symbol w/ value is written to `{symbol}_choice`.
    pub fn combo_box(&mut self, label: impl AsRef<str>, with_symbol: impl AsRef<str>, ui: &Ui) {
        let choices = self.as_ref().find_symbol_values(&with_symbol);
 
        let id = self.entity;
        let label = label.as_ref();
        let symbol = with_symbol.as_ref();
        let key = format!("{symbol}_choice_index");
        let mut choice = 0;
        if let Some(Value::Int(index)) = self.as_mut().find_attr_value_mut(&key) {
            choice = *index as usize;
            if ui.combo(format!("{label} {id}"), &mut choice, &choices, |(name, _)| {
                Cow::from(name.as_str().trim_end_matches(&format!("::{symbol}")))
            } ) {
                *index = choice as i32;
            }
        } else {
            self.as_mut().add_int_attr(key, 0);
        }

        if let Some((_, value)) = choices.get(choice) {
            self.as_mut().with(format!("{symbol}_choice"), value.clone());
        }
     }
}

#[test]
fn test_attribute_graph() {
    let mut test_graph = AttributeGraph::default();

    test_graph
        .with("test_value", Value::Int(10))
        .with("test_float", Value::Float(10.0));

    assert!(test_graph.contains_attribute("test_value"));
    assert_eq!(
        test_graph.find_attr_value("test_value"),
        Some(&Value::Int(10))
    );
    assert!(test_graph.contains_attribute("test_float"));
    assert_eq!(
        test_graph.find_attr_value("test_float"),
        Some(&Value::Float(10.0))
    );
}

impl Display for AttributeGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

impl AsRef<AttributeGraph> for AttributeGraph {
    fn as_ref(&self) -> &AttributeGraph {
        self
    }
}

impl AsMut<AttributeGraph> for AttributeGraph {
    fn as_mut(&mut self) -> &mut AttributeGraph {
        self
    }
}

impl From<Entity> for AttributeGraph {
    fn from(entity: Entity) -> Self {
        Self::from(entity.id())
    }
}

impl From<u32> for AttributeGraph {
    fn from(entity_id: u32) -> Self {
        AttributeGraph {
            entity: entity_id,
            index: BTreeMap::default(),
        }
    }
}

#[derive(Debug)]
pub enum AttributeGraphErrors {
    UnknownEvent,
    NotEnoughArguments,
    WrongArugment,
    IncorrectMessageFormat,
    CannotImportEmptyAttribute,
    EmptyMessage,
}
