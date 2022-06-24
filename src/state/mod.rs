use crate::{editor::unique_title, RuntimeDispatcher, RuntimeState};
use atlier::system::{Attribute, Value};
use imgui::TableFlags;
use logos::Logos;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use specs::{storage::HashMapStorage, Component, Entity};
use std::{
    collections::{hash_map::DefaultHasher, BTreeMap},
    fmt::Display,
    fs,
    hash::{Hash, Hasher},
    str::from_utf8, path::PathBuf,
};

/// Attribute graph is a component that indexes attributes for an entity
/// It is designed to be a general purpose enough to be the common element of runtime state storage
#[derive(Debug, Default, Component, Clone, Hash, Serialize, Deserialize, PartialEq, PartialOrd)]
#[storage(HashMapStorage)]
pub struct AttributeGraph {
    entity: u32,
    index: BTreeMap<String, Attribute>,
}

impl TryInto<Attribute> for AttributeGraph {
    type Error = ();

    fn try_into(self) -> Result<Attribute, Self::Error> {
        if let Some(saved) = self.save() {
            Ok(Attribute::new(
                self.entity,
                format!("{}.graph", self.hash_code()),
                Value::BinaryVector(saved.as_bytes().to_vec()),
            ))
        } else {
            Err(())
        }
    }
}

impl From<Attribute> for AttributeGraph {
    fn from(attr: Attribute) -> Self {
        let graph = AttributeGraph::default();
        if let Value::BinaryVector(graph_content) = attr.value() {
            let graph_content = String::from_utf8(graph_content.to_vec()).unwrap_or_default();
            let graph = graph.load(graph_content);
            println!("{}", graph.save().expect("exists"));
            return graph;
        }
        eprintln!(
            "could not load attribute: {:?}, returning empty graph",
            attr
        );
        graph
    }
}

impl AttributeGraph {
    /// ends block mode, and puts the graph back to its initial root 
    pub fn root_mut(&mut self) -> &mut Self {
        self.dispatch_mut("```").expect("should be able to end block mode");
        self
    }

    /// writes a binary vector in graph with attr_name to path
    /// tries to create parent directories in path before writing the file
    pub fn write_file_as(&self, path: impl AsRef<str>, attr_name: impl AsRef<str>) -> std::io::Result<()> {
        let path_buf = PathBuf::from(path.as_ref());

        if let Some(parent) = path_buf.parent() {
            fs::create_dir_all(parent)?;
        }

        if let Some(Value::BinaryVector(contents)) = self.find_attr_value(attr_name.as_ref()) {
            fs::write(path.as_ref(), contents)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound, 
                format!("could not find binary attribute {}", attr_name.as_ref()))
            )
        }
    }

    /// writes a binary vector in graph as path to path
    /// meant to be used with add_file
    pub fn write_file(&self, path: impl AsRef<str>) -> std::io::Result<()> {
        self.write_file_as(path.as_ref(), path.as_ref())
    }

    /// adds a file from path to a binary attr
    pub fn add_file(&mut self, path: impl AsRef<str>) {
        match fs::read_to_string(path.as_ref()) {
            Ok(content) => {
                self.add_file_with(path, content);
            },
            Err(err) => {
                eprintln!("file not added {}", err);
            },
        }
    }

    /// adds a file with path and content
    pub fn add_file_with(&mut self, path: impl AsRef<str>, content: impl AsRef<str>) {
        self.add_binary_attr(path, content.as_ref());
    }

    /// finds a file attribute that was added with add_file
    pub fn find_file(&self, path: impl AsRef<str>) -> Option<Vec<u8>> {
        if let Some(Value::BinaryVector(content)) = self.find_attr_value(path) {
            Some(content.to_vec())
        } else {
            None
        }
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
    pub fn add_message(&mut self, name: impl AsRef<str>, symbol: impl AsRef<str>, message: impl AsRef<str>) {
        self.define(name.as_ref(), symbol.as_ref())
            .edit_as(Value::BinaryVector(message.as_ref().as_bytes().to_vec()));
    }

    /// finds and applies all messages to graph
    pub fn apply(&mut self, symbol: impl AsRef<str>) {
        for (name, value) in self.take_symbol_values(symbol.as_ref()) {
            println!("Symbol '{}' {}", symbol.as_ref(), name);
            if let Value::BinaryVector(content) = value {
                if let Some(content) = from_utf8(&content).ok() {
                    println!("Applying {}\n{}", symbol.as_ref(), content);

                    match self.batch_mut(content) {
                        Ok(_) => {
                            println!("Applied {} {}", symbol.as_ref(), name);
                            self.find_remove(name);
                        }
                        Err(_) => {
                            eprintln!("could not apply events");
                        }
                    }
                }
            }
        }
    }

    /// commits each attribute and saves to attribute
    pub fn to_attribute(&self) -> Option<Attribute> {
        self.clone().try_into().ok()
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
        let mut loading = AttributeGraph::default();

        if loading.from_file(&path).is_ok() {
            let loaded = loading.define("src", "file");
            loaded.edit_as(Value::TextBuffer(path.as_ref().to_string()));

            Some(loading)
        } else {
            None
        }
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

    /// Define a symbol attribute.
    pub fn define(&mut self, name: impl AsRef<str>, symbol: impl AsRef<str>) -> &mut Attribute {
        let symbol_name = format!("{}::{}", name.as_ref(), symbol.as_ref());
        let symbol_value = format!("{}::", symbol.as_ref());
        self.add_symbol(&symbol_name, symbol_value);

        let defined = self.find_attr_mut(&symbol_name).expect("just added");
        defined.edit_as(Value::Empty);
        defined
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
        if let Some(Value::Float(width)) = self.find_attr_value("edit_width") {
            ui.set_next_item_width(*width);
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
                if fs::write(&file_name, self.save().unwrap_or_default()).is_ok() {
                    println!("Saved output to {}", file_name);
                }
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
                    if self.from_file(&file_source).is_ok() {
                        println!("Reloaded {}", &file_source);
                    }
                }
            }

            ui.separator();
            token.end()
        }

        if let Some(token) = ui.begin_menu("Edit") {
            if let Some(token) = ui.begin_menu("Add new attribute") {
                if imgui::MenuItem::new("Text").build(ui) {
                    self.add_text_attr(unique_title("text"), "");
                }

                if imgui::MenuItem::new("Bool").build(ui) {
                    self.add_bool_attr(unique_title("bool"), false);
                }

                if imgui::MenuItem::new("Int").build(ui) {
                    self.add_int_attr(unique_title("int"), 0);
                }

                if imgui::MenuItem::new("Int pair").build(ui) {
                    self.add_int_pair_attr(unique_title("int_pair"), &[0, 0]);
                }

                if imgui::MenuItem::new("Float").build(ui) {
                    self.add_float_attr(unique_title("float"), 0.0);
                }

                if imgui::MenuItem::new("Float pair").build(ui) {
                    self.add_float_pair_attr(unique_title("float_pair"), &[0.0, 0.0]);
                }

                if imgui::MenuItem::new("Empty").build(ui) {
                    self.add_empty_attr(unique_title("empty"));
                }

                token.end();
            }

            ui.separator();
            if let Some(token) = ui.begin_menu("Remove attribute..") {
                for attr in self.clone().iter_attributes() {
                    if imgui::MenuItem::new(format!("{}", attr)).build(ui) {
                        self.remove(attr);
                    }
                }

                token.end();
            }

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
            eprintln!("Warning: No-Op, Trying to import an attribute that is not external to this graph, add this attribute by value instead");
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
        self.find_attr(with_name).and_then(|a| Some(a.value()))
    }

    /// Finds a text value of an attribute
    pub fn find_text(&self, with_name: impl AsRef<str>) -> Option<String> {
        self.find_attr_value(with_name).and_then(|n| {
            if let Value::TextBuffer(text) = n {
                Some(text.to_string())
            } else {
                None
            }
        })
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

    /// Returns self with an empty attribute w/ name.
    pub fn with_empty(&mut self, name: impl AsRef<str>) -> &mut Self {
        self.with(name, Value::Empty)
    }

    /// Returns self with a symbol attribute w/ name.
    pub fn with_symbol(&mut self, name: impl AsRef<str>, symbol: impl AsRef<str>) -> &mut Self {
        self.with(name, Value::Symbol(symbol.as_ref().to_string()))
    }

    /// Returns self with a binary attribute w/ name
    pub fn with_binary(&mut self, name: impl AsRef<str>, binary: impl Into<Vec<u8>>) -> &mut Self {
        self.with(name, Value::BinaryVector(binary.into()))
    }

    /// Returns self with a text buffer attribute w/ name.
    pub fn with_text(&mut self, name: impl AsRef<str>, init_value: impl AsRef<str>) -> &mut Self {
        self.with(name, Value::TextBuffer(init_value.as_ref().to_string()))
    }

    /// Returns self with an int attribute w/ name.
    pub fn with_int(&mut self, name: impl AsRef<str>, init_value: i32) -> &mut Self {
        self.with(name, Value::Int(init_value))
    }

    /// Returns self with a float attribute w/ name.
    pub fn with_float(&mut self, name: impl AsRef<str>, init_value: f32) -> &mut Self {
        self.with(name, Value::Float(init_value))
    }

    /// Returns self with a bool attribute w/ name.
    pub fn with_bool(&mut self, name: impl AsRef<str>, init_value: bool) -> &mut Self {
        self.with(name, Value::Bool(init_value))
    }

    /// Returns self with a float pair attribute w/ name.
    pub fn with_float_pair(&mut self, name: impl AsRef<str>, init_value: &[f32; 2]) -> &mut Self {
        self.with(name, Value::FloatPair(init_value[0], init_value[1]))
    }

    /// Returns self with an int pair attribute w/ name.
    pub fn with_int_pair(&mut self, name: impl AsRef<str>, init_value: &[i32; 2]) -> &mut Self {
        self.with(name, Value::IntPair(init_value[0], init_value[1]))
    }

    /// Returns self with an int range attribute w/ name.
    pub fn with_int_range(&mut self, name: impl AsRef<str>, init_value: &[i32; 3]) -> &mut Self {
        self.with(
            name,
            Value::IntRange(init_value[0], init_value[1], init_value[2]),
        )
    }

    /// Returns self with a float range attribute w/ name.
    pub fn with_float_range(&mut self, name: impl AsRef<str>, init_value: &[f32; 3]) -> &mut Self {
        self.with(
            name,
            Value::FloatRange(init_value[0], init_value[1], init_value[2]),
        )
    }

    /// Add's a value and returns self to make these api's chainable
    pub fn with(&mut self, name: impl AsRef<str>, value: Value) -> &mut Self {
        self.update(move |g| match value {
            Value::Empty => {
                g.add_empty_attr(name);
            }
            Value::Symbol(symbol) => {
                g.add_symbol(name, symbol);
            }
            Value::TextBuffer(text_buffer) => {
                g.add_text_attr(name, text_buffer);
            }
            Value::Float(init_value) => {
                g.add_float_attr(name, init_value);
            }
            Value::Int(init_value) => {
                g.add_int_attr(name, init_value);
            }
            Value::Bool(init_value) => {
                g.add_bool_attr(name, init_value);
            }
            Value::IntPair(e0, e1) => {
                g.add_int_pair_attr(name, &[e0, e1]);
            }
            Value::FloatPair(e0, e1) => {
                g.add_float_pair_attr(name, &[e0, e1]);
            }
            Value::FloatRange(value, min, max) => {
                g.add_float_range_attr(name, &[value, min, max]);
            }
            Value::IntRange(value, min, max) => {
                g.add_int_range_attr(name, &[value, min, max]);
            }
            Value::BinaryVector(init_value) => {
                g.add_binary_attr(name, init_value);
            }
            Value::Reference(init_value) => {
                g.add_reference(name, init_value);
            }
        })
    }

    /// Adds a reference attribute w/ init_value and w/ name to index for entity.
    pub fn add_reference(&mut self, name: impl AsRef<str>, init_value: impl Into<u64>) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::Reference(init_value.into()),
        ));
    }

    /// Adds a symbol attribute w/ symbol and w/ name to index for entity.
    pub fn add_symbol(&mut self, name: impl AsRef<str>, symbol: impl AsRef<str>) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::Symbol(symbol.as_ref().to_string()),
        ));
    }

    /// Adds an empty attribute w/ name to index for entity.
    pub fn add_empty_attr(&mut self, name: impl AsRef<str>) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::Empty,
        ));
    }

    /// Adds a binary vector attribute w/ name and w/ init_value for entity.
    pub fn add_binary_attr(&mut self, name: impl AsRef<str>, init_value: impl Into<Vec<u8>>) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::BinaryVector(init_value.into()),
        ));
    }

    /// Adds a text buffer attribute w/ name and w/ init_value for entity.
    pub fn add_text_attr(&mut self, name: impl AsRef<str>, init_value: impl AsRef<str>) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::TextBuffer(init_value.as_ref().to_string()),
        ));
    }

    /// Adds an int attribute w/ name and w/ init_value for entity.
    pub fn add_int_attr(&mut self, name: impl AsRef<str>, init_value: i32) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::Int(init_value),
        ));
    }

    /// Adds an float attribute w/ name and w/ init_value for entity.
    pub fn add_float_attr(&mut self, name: impl AsRef<str>, init_value: f32) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::Float(init_value),
        ));
    }

    /// Adds a bool attribute w/ name and w/ init_value for entity.
    pub fn add_bool_attr(&mut self, name: impl AsRef<str>, init_value: bool) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::Bool(init_value),
        ));
    }

    /// Adds a float pair attribute w/ name and w/ init_value for entity.
    pub fn add_float_pair_attr(&mut self, name: impl AsRef<str>, init_value: &[f32; 2]) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::FloatPair(init_value[0], init_value[1]),
        ));
    }

    /// Adds an int pair attribute w/ name and w/ init_value for entity.
    pub fn add_int_pair_attr(&mut self, name: impl AsRef<str>, init_value: &[i32; 2]) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::IntPair(init_value[0], init_value[1]),
        ));
    }

    /// Adds an int range attribute w/ name and w/ init_value for entity.
    pub fn add_int_range_attr(&mut self, name: impl AsRef<str>, init_value: &[i32; 3]) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::IntRange(init_value[0], init_value[1], init_value[2]),
        ));
    }

    /// Adds an float range attribute w/ name and w/ init_value for entity.
    pub fn add_float_range_attr(&mut self, name: impl AsRef<str>, init_value: &[f32; 3]) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::FloatRange(init_value[0], init_value[1], init_value[2]),
        ));
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

    pub fn add_attribute(&mut self, attr: Attribute) {
        if !attr.is_stable() {
            if let Value::Symbol(symbol) = attr.value() {
                let parts: Vec<&str> = symbol.split("::").collect();
                if let Some(first_part) = parts.get(0) {
                    let key = attr
                        .to_string()
                        .replace(&format!("{}::", first_part), &symbol);
                    self.index.insert(key, attr);
                } else {
                    eprintln!("symbol value was empty");
                }
            } else {
                self.index.insert(attr.to_string(), attr);
            }
        } else {
            self.index.insert(attr.to_string(), attr);
        }
    }

    fn update(&mut self, func: impl FnOnce(&mut Self)) -> &mut Self {
        (func)(self);
        self
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

impl RuntimeState for AttributeGraph {
    type Dispatcher = Self;

    /// Try to serialize self to string in .ron format.
    fn save(&self) -> Option<String> {
        ron::ser::to_string_pretty(self, PrettyConfig::new()).ok()
    }

    /// Try to load self from .ron formatted string.
    fn load(&self, init: impl AsRef<str>) -> Self {
        if let Some(state) = ron::from_str(init.as_ref()).ok() {
            state
        } else {
            Self::default()
        }
    }

    /// Returns dispatcher for runtime state
    fn dispatcher(&self) -> &Self::Dispatcher {
        self
    }

    /// Returns mutable dispatcher for runtime state
    fn dispatcher_mut(&mut self) -> &mut Self::Dispatcher {
        self
    }

    fn setup_runtime(&mut self, runtime: &mut crate::Runtime<Self>) {
        runtime.with_call("dispatch", |s, e| {
            if let Some(msg) = e.and_then(|e| e.read_payload()) {
                match s.dispatch(&msg) {
                    Ok(next) => (Some(next), "{ ok;; }".to_string()),
                    Err(_) => (None, "{ error;; }".to_string()),
                }
            } else {
                (None, "{ exit;; }".to_string())
            }
        });
    }
}

impl RuntimeDispatcher for AttributeGraph {
    type Error = AttributeGraphErrors;

    /// dispatch_mut is a function that should take a string message that can mutate state
    /// and returns a result
    fn dispatch_mut(&mut self, msg: impl AsRef<str>) -> Result<(), Self::Error> {
        let mut event_lexer = AttributeGraphEvents::lexer(msg.as_ref());

        match event_lexer.next() {
            Some(event) => match event {
                AttributeGraphEvents::Add => self.on_add(event_lexer.remainder()),
                AttributeGraphEvents::FindRemove => self.on_find_remove(event_lexer.remainder()),
                AttributeGraphEvents::Import => self.on_import(event_lexer.remainder()),
                AttributeGraphEvents::Copy => self.on_copy(event_lexer.remainder()),
                AttributeGraphEvents::Define => self.on_define(event_lexer.remainder()),
                AttributeGraphEvents::Apply => self.on_apply(event_lexer.remainder()),
                AttributeGraphEvents::Edit => self.on_edit(event_lexer.remainder()),
                AttributeGraphEvents::From => self.on_from(event_lexer.remainder()),
                AttributeGraphEvents::To => self.on_to(event_lexer.remainder()),
                AttributeGraphEvents::Publish => self.on_publish(event_lexer.remainder()),
                AttributeGraphEvents::BlockDelimitter => self.on_block(event_lexer.remainder()),
                AttributeGraphEvents::Comment => Ok(()),
                AttributeGraphEvents::Error => Err(AttributeGraphErrors::UnknownEvent),
            },
            None => Err(AttributeGraphErrors::EmptyMessage),
        }
    }
}

/// These are handlers for dispatched messages
impl AttributeGraph {
    fn on_publish(&mut self, msg: impl AsRef<str>) -> Result<(), AttributeGraphErrors> {
        let mut element_lexer = AttributeGraphElements::lexer(msg.as_ref());

        match element_lexer.next() {
            Some(AttributeGraphElements::Symbol(attr_name)) => {
                self.find_update_attr(attr_name, |a| a.edit_self());
                Ok(())
            }
            Some(_) => Err(AttributeGraphErrors::WrongArugment),
            None => Err(AttributeGraphErrors::NotEnoughArguments),
        }
    }

    fn next_block(&mut self, with_name: impl AsRef<str>, symbol_name: impl AsRef<str>) -> u32 {
        if let None = self.find_parent_block() {
            let parent_entity = self.entity() as i32;

            self.define("parent", "block")
                .edit_as(Value::Int(parent_entity));
        }

        if let Some(mut last_block) = self.find_last_block() {
            if let Some((_, Value::Int(last_block_id))) = last_block.take_transient() {
                self.entity = last_block_id as u32;
            }
        }

        if let Some(block_id) = self.find_block_id(with_name, symbol_name) {
            self.entity = block_id;
        } else {
            self.entity += 1;
        }

        self.entity
    }

    fn end_block_mode(&mut self) {
        let last_block_id = self.entity.clone();
        if let Some(parent_block) = self.find_parent_block() {
            if let Some((_, Value::Int(parent_entity))) = parent_block.transient() {
                self.entity = *parent_entity as u32;

                self.define("last", "block")
                    .edit_as(Value::Int(last_block_id as i32));
            }
        }
    }

    fn start_block_mode(&mut self, block_name: impl AsRef<str>, block_symbol: impl AsRef<str>) {
        let block_id = self.next_block(&block_name, &block_symbol);

        let block = self.define(&block_name, &block_symbol);
        block.edit_as(Value::Symbol(format!(
            "{}::{}",
            block_symbol.as_ref(),
            "block"
        )));
        block.commit();
        block.edit_as(Value::Int(block_id as i32));

        self.with_text("block_name", block_name)
            .with_text("block_symbol", block_symbol);
    }

    fn on_block(&mut self, msg: impl AsRef<str>) -> Result<(), AttributeGraphErrors> {
        let mut element_lexer = AttributeGraphElements::lexer(msg.as_ref());

        match (element_lexer.next(), element_lexer.next()) {
            (
                Some(AttributeGraphElements::Name(block_name)),
                Some(AttributeGraphElements::Symbol(block_symbol)),
            )
            | (
                Some(AttributeGraphElements::Symbol(block_name)),
                Some(AttributeGraphElements::Symbol(block_symbol)),
            ) => {
                self.start_block_mode(block_name, block_symbol);
            }
            (Some(AttributeGraphElements::Symbol(block_symbol)), _) => {
                if let Some(block_name) = self.find_text("block_name") {
                    self.start_block_mode(block_name, block_symbol);
                }
            }
            _ => {
                self.end_block_mode();
            }
        }
        Ok(())
    }

    fn on_edit(&mut self, msg: impl AsRef<str>) -> Result<(), AttributeGraphErrors> {
        let mut element_lexer = AttributeGraphElements::lexer(msg.as_ref());
        match (
            element_lexer.next(),
            element_lexer.next(),
            element_lexer.next(),
        ) {
            (
                Some(AttributeGraphElements::Symbol(name)),
                Some(AttributeGraphElements::Symbol(new_name)),
                Some(value),
            ) => match value {
                AttributeGraphElements::Text(value)
                | AttributeGraphElements::Int(value)
                | AttributeGraphElements::Bool(value)
                | AttributeGraphElements::IntPair(value)
                | AttributeGraphElements::IntRange(value)
                | AttributeGraphElements::Float(value)
                | AttributeGraphElements::FloatPair(value)
                | AttributeGraphElements::FloatRange(value)
                | AttributeGraphElements::BinaryVector(value) => {
                    if let Some(attr) = self.find_attr_mut(name) {
                        attr.edit((new_name, value));
                    }
                    Ok(())
                }
                AttributeGraphElements::Empty => {
                    if let Some(attr) = self.find_attr_mut(name) {
                        attr.edit((new_name, Value::Empty));
                    }
                    Ok(())
                }
                AttributeGraphElements::Entity(_) => todo!("value type unknown"),
                AttributeGraphElements::Symbol(_) | AttributeGraphElements::Name(_) => {
                    todo!("unrecognized element")
                }
                AttributeGraphElements::Error => todo!("error parsing next value"),
            },
            (Some(AttributeGraphElements::Symbol(name)), Some(value), _) => match value {
                AttributeGraphElements::Text(value)
                | AttributeGraphElements::Int(value)
                | AttributeGraphElements::Bool(value)
                | AttributeGraphElements::IntPair(value)
                | AttributeGraphElements::IntRange(value)
                | AttributeGraphElements::Float(value)
                | AttributeGraphElements::FloatPair(value)
                | AttributeGraphElements::FloatRange(value)
                | AttributeGraphElements::BinaryVector(value) => {
                    if let Some(attr) = self.find_attr_mut(&name) {
                        let parts: Vec<&str> = name.split("::").collect();
                        if let Some(name) = parts.first() {
                            attr.edit((name.to_string(), value));
                        }
                    }
                    Ok(())
                }
                AttributeGraphElements::Empty => {
                    if let Some(attr) = self.find_attr_mut(&name) {
                        let parts: Vec<&str> = name.split("::").collect();
                        if let Some(name) = parts.first() {
                            attr.edit((name.to_string(), Value::Empty));
                        }
                    }
                    Ok(())
                }
                AttributeGraphElements::Entity(_) => todo!("value type unknown"),
                AttributeGraphElements::Symbol(_) | AttributeGraphElements::Name(_) => {
                    todo!("unrecognized element")
                }
                AttributeGraphElements::Error => todo!("error parsing next value"),
            },
            _ => Err(AttributeGraphErrors::NotEnoughArguments),
        }
    }

    fn on_apply(&mut self, msg: impl AsRef<str>) -> Result<(), AttributeGraphErrors> {
        let mut element_lexer = AttributeGraphElements::lexer(msg.as_ref());
        match (element_lexer.next(), element_lexer.next()) {
            (
                Some(AttributeGraphElements::Symbol(name)),
                Some(AttributeGraphElements::Symbol(symbol)),
            ) => {
                let symbol_attr_name = format!("{}::{}", name, symbol);
                if let Some(transient) = self
                    .find_attr_mut(symbol_attr_name)
                    .and_then(|a| a.take_transient())
                    .and_then(|a| Some(a.clone()))
                {
                    // This method will also update the key if the name happens to change
                    if !self.find_update_attr(name, |to_edit| {
                        to_edit.edit(transient.clone());
                        to_edit.commit();
                    }) {
                        // If the attribute didn't already exist, this will create a new attribute
                        // with the name in the transient
                        let (name, value) = transient;
                        self.with(name, value.clone());
                    }
                }

                Ok(())
            }
            (Some(AttributeGraphElements::Symbol(attr_name)), _) => {
                if self.find_update_attr(attr_name, |a| a.commit()) {
                    Ok(())
                } else {
                    // todo
                    Ok(())
                }
            }
            _ => Err(AttributeGraphErrors::NotEnoughArguments),
        }
    }

    fn on_define(&mut self, msg: impl AsRef<str>) -> Result<(), AttributeGraphErrors> {
        let mut element_lexer = AttributeGraphElements::lexer(msg.as_ref());
        match (element_lexer.next(), element_lexer.next()) {
            (
                Some(AttributeGraphElements::Symbol(name)),
                Some(AttributeGraphElements::Symbol(symbol)),
            ) => {
                self.define(name, symbol);
                Ok(())
            }
            _ => Err(AttributeGraphErrors::NotEnoughArguments),
        }
    }

    fn on_add(&mut self, msg: impl AsRef<str>) -> Result<(), AttributeGraphErrors> {
        let mut element_lexer = AttributeGraphElements::lexer(msg.as_ref());
        match (element_lexer.next(), element_lexer.next()) {
            (Some(AttributeGraphElements::Symbol(name)), Some(value)) => match value {
                AttributeGraphElements::Text(value)
                | AttributeGraphElements::Int(value)
                | AttributeGraphElements::Bool(value)
                | AttributeGraphElements::IntPair(value)
                | AttributeGraphElements::IntRange(value)
                | AttributeGraphElements::Float(value)
                | AttributeGraphElements::FloatPair(value)
                | AttributeGraphElements::FloatRange(value)
                | AttributeGraphElements::BinaryVector(value) => {
                    self.with(name, value);
                    Ok(())
                }
                AttributeGraphElements::Empty => {
                    self.with_empty(name);
                    Ok(())
                }
                AttributeGraphElements::Entity(_) => todo!("value type unknown"),
                AttributeGraphElements::Symbol(_) | AttributeGraphElements::Name(_) => {
                    todo!("unrecognized element")
                }
                AttributeGraphElements::Error => todo!("error parsing next value"),
            },
            _ => Err(AttributeGraphErrors::NotEnoughArguments),
        }
    }

    fn on_find_remove(&mut self, msg: impl AsRef<str>) -> Result<(), AttributeGraphErrors> {
        let mut element_lexer = AttributeGraphElements::lexer(msg.as_ref());
        match element_lexer.next() {
            Some(AttributeGraphElements::Symbol(attr_name)) => {
                if let Some(removed) = self.find_remove(&attr_name) {
                    eprintln!("Removed {}", removed);
                } else {
                    eprintln!("Attribute not found {}", &attr_name);
                }
                Ok(())
            }
            _ => Err(AttributeGraphErrors::NotEnoughArguments),
        }
    }

    fn on_import(&mut self, msg: impl AsRef<str>) -> Result<(), AttributeGraphErrors> {
        let mut element_lexer = AttributeGraphElements::lexer(msg.as_ref());
        match (
            element_lexer.next(),
            element_lexer.next(),
            element_lexer.next(),
        ) {
            (
                Some(AttributeGraphElements::Entity(entity)),
                Some(AttributeGraphElements::Symbol(name)),
                Some(value),
            ) => match value {
                AttributeGraphElements::Text(value)
                | AttributeGraphElements::Int(value)
                | AttributeGraphElements::Bool(value)
                | AttributeGraphElements::IntPair(value)
                | AttributeGraphElements::IntRange(value)
                | AttributeGraphElements::Float(value)
                | AttributeGraphElements::FloatPair(value)
                | AttributeGraphElements::FloatRange(value)
                | AttributeGraphElements::BinaryVector(value) => {
                    self.import_attribute(&Attribute::new(entity, name, value));
                    Ok(())
                }
                AttributeGraphElements::Empty => {
                    Err(AttributeGraphErrors::CannotImportEmptyAttribute)
                }
                AttributeGraphElements::Entity(_)
                | AttributeGraphElements::Name(_)
                | AttributeGraphElements::Symbol(_)
                | AttributeGraphElements::Error => {
                    Err(AttributeGraphErrors::IncorrectMessageFormat)
                }
            },
            _ => Err(AttributeGraphErrors::NotEnoughArguments),
        }
    }

    fn on_copy(&mut self, msg: impl AsRef<str>) -> Result<(), AttributeGraphErrors> {
        let mut element_lexer = AttributeGraphElements::lexer(msg.as_ref());
        match (element_lexer.next(), element_lexer.next()) {
            (
                Some(AttributeGraphElements::Entity(entity)),
                Some(AttributeGraphElements::Symbol(name)),
            ) => {
                if let Some(imported) = self.find_imported_graph(entity) {
                    if let Some(to_copy) = imported.find_attr(name) {
                        self.copy_attribute(to_copy);
                    }
                }
                Ok(())
            }
            (Some(AttributeGraphElements::Entity(entity)), None) => {
                if let Some(imported) = self.find_imported_graph(entity) {
                    self.copy(&imported);
                }
                Ok(())
            }
            _ => Err(AttributeGraphErrors::NotEnoughArguments),
        }
    }

    fn on_from(&mut self, msg: impl AsRef<str>) -> Result<(), AttributeGraphErrors> {
        // Example
        // from block_name block_symbol attr_name -> transient attribute
        // from block_name block_symbol attr_name #expect
        // from block_symbol attr_name
        let mut element_lexer = AttributeGraphElements::lexer(msg.as_ref());

        match (
            element_lexer.next(),
            element_lexer.next(),
            element_lexer.next(),
        ) {
            (
                Some(AttributeGraphElements::Symbol(block_name)),
                Some(AttributeGraphElements::Symbol(block_symbol)),
                Some(AttributeGraphElements::Symbol(attr_name)),
            )
            | (
                Some(AttributeGraphElements::Name(block_name)),
                Some(AttributeGraphElements::Symbol(block_symbol)),
                Some(AttributeGraphElements::Symbol(attr_name)),
            ) => {
                self.from_block(block_name, block_symbol, attr_name);
            }
            (
                Some(AttributeGraphElements::Symbol(block_symbol)),
                Some(AttributeGraphElements::Symbol(attr_name)),
                _,
            ) => {
                if let Some(block_name) = self.find_text("block_name") {
                    self.from_block(block_name, block_symbol, attr_name);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn on_to(&mut self, msg: impl AsRef<str>) -> Result<(), AttributeGraphErrors> {
        let mut element_lexer = AttributeGraphElements::lexer(msg.as_ref());

        match (element_lexer.next(), element_lexer.next()) {
            (
                Some(AttributeGraphElements::Symbol(block_symbol)),
                Some(AttributeGraphElements::Symbol(attr_name)),
            ) => {
                self.to_block(block_symbol, attr_name);
                Ok(())
            }
            (Some(_), Some(_)) => Err(AttributeGraphErrors::WrongArugment),
            _ => Ok(()),
        }
    }
}

#[test]
fn test_attribute_graph_block_dispatcher() {
    let mut graph = AttributeGraph::from(0);

    let test = r#"
    ```
    ```
    "#;

    assert!(graph.batch_mut(test).is_ok());
    assert_eq!(graph.entity, 0);

    let test = r#"
    ``` demo node
    add demo_node_title .TEXT hello demo node
    ``` demo2 node
    add demo_node_title .TEXT hello demo ndoe 2
    ```
    "#;

    assert!(graph.batch_mut(test).is_ok());
    assert_eq!(graph.entity, 0);

    let test = r#"
    ``` demo node
    add demo_node_title .TEXT hello demo node
    ``` demo2 node
    add demo_node_title .TEXT hello demo ndoe 2
    ```
    "#;

    assert!(graph.batch_mut(test).is_ok());
    assert_eq!(graph.entity, 0);
    assert_eq!(graph.find_blocks("node").len(), 2);

    let test = r#"
    ``` demo node
    add demo_node_title .TEXT hello demo node
    ``` demo2 node
    add demo_node_title .TEXT hello demo ndoe 2
    ```
    "#;

    assert!(graph.batch_mut(test).is_ok());
    assert_eq!(graph.entity, 0);
    assert_eq!(graph.find_blocks("node").len(), 2);

    let test = r#"
    ``` demo3 node
    add demo_node_title .TEXT hello demo node
    ``` demo4 node
    add demo_node_title .TEXT hello demo ndoe 2
    ```
    "#;

    assert!(graph.batch_mut(test).is_ok());
    assert_eq!(graph.entity, 0);
    assert_eq!(graph.find_blocks("node").len(), 4);

    println!("{}", graph.save().expect(""));
    assert_eq!(Some(1), graph.find_block_id("demo", "node"));
    assert!(graph
        .find_block("demo3", "node")
        .and_then(|a| Some(a.entity() == 3))
        .expect("should return the correct block"));

    let test = r#"
    ``` demo3 node2
    add demo_node_title .TEXT hello demo node
    publish demo_node_title
    ``` demo4 node3
    add demo_node_title .TEXT hello demo ndoe 2
    from demo3 node2 demo_node_title
    ``` demo4 node6
    add demo_node_title .TEXT testing to_demo node
    to node demo_node_title
    to node2 demo_node_title
    publish demo_node_title
    ``` demo4 window
    add demo_node_title .EMPTY
    from node6 demo_node_title
    apply demo_node_title
    from demo_doesnt_exist demo_node_title
    ```
    "#;

    assert!(graph.batch_mut(test).is_ok());
    assert_eq!(graph.entity, 0);
    assert_eq!(graph.iter_blocks().count(), 8);
    println!("printing all blocks");
    for b in graph.iter_blocks() {
        println!("{}", b.save().expect(""));
    }

    let check_from = graph.clone().find_block("demo4", "node3").expect("exists");
    println!("{:#?}", check_from);

    let check_from = check_from.find_attr("demo_node_title").expect("exists");

    let check_from = (check_from.name().to_string(), check_from.value().clone());

    assert_eq!(
        (
            "demo_node_title".to_string(),
            Value::TextBuffer("hello demo node".to_string())
        ),
        check_from
    );

    let check_to = graph.clone().find_block("demo4", "node").expect("exists");
    println!("{:#?}", check_to);
    println!("{:#?}", check_to.find_symbol_values("link"));

    let check_to = check_to
        .find_attr("demo_node_title")
        .expect("exists")
        .transient();
    assert_eq!(
        Some(&(
            "demo_node_title".to_string(),
            Value::TextBuffer("testing to_demo node".to_string())
        )),
        check_to
    );

    let check_from_other = graph.clone().find_block("demo4", "window").expect("exists");
    println!("{:#?}", check_from_other);

    let check_from_other = check_from_other.find_attr_value("demo_node_title");
    assert_eq!(
        Some(&Value::TextBuffer("testing to_demo node".to_string())),
        check_from_other
    );

    for demo4_block in graph.find_blocks_for("demo4") {
        println!("{:#?}", demo4_block.find_text("block_name"));
        println!("{:#?}", demo4_block.find_text("block_symbol"));
        println!("{:#?}", demo4_block);
    }
}

#[test]
fn test_block_context() {
    let test = r#"
    ``` demo5 node
    add node_title .TEXT really cool node3
    ``` form 
    add form_text .EMPTY
    edit form_text .TEXT placeholder
    ``` demo2 node
    add node_title .TEXT really cool node3
    add coolness2 .EMPTY
    add input_label .TEXT coolness2
    ``` demo2 form
    add form_text .EMPTY
    edit form_text .TEXT placeholder
    ``` demo3 node
    add node_title .TEXT really cool node3
    ``` demo3 form 
    add form_text .EMPTY
    edit form_text .TEXT placeholder
    ```
    ``` #tasks.yaml file
    ``` node
    ``` form
    ```

    ``` #Main.elm file
    ```
    "#;

    let mut graph = AttributeGraph::from(0);

    assert!(graph.batch_mut(test).is_ok());
    println!("{}", graph.save().expect("exists"));

    for mut node in graph.find_blocks("node") {
        graph.include_block(&mut node, "form");
        println!("{}", node.save().expect("exists"));

        if let Some(form) = node.find_block("", "form") {
            println!("{}", form.save().expect("exists"));
        }
    }

    let mut tasks_yaml = graph.find_block("tasks.yaml", "file").expect("exists");
    graph.include_block(&mut tasks_yaml, "node");
    graph.include_block(&mut tasks_yaml, "form");
    println!("{}", tasks_yaml.save().expect("exists"));

    let main_elm = graph.find_block("Main.elm", "file").expect("exists");
    println!("{}", main_elm.save().expect("exists"));

    let commited_main_elm = main_elm.commit(|_|{});
    let attr: Option<Attribute> = commited_main_elm.to_attribute();
    println!("{:?}", attr);

    if let Some(attr) = attr {
        let reloaded = AttributeGraph::from(attr);
        assert_eq!(reloaded, commited_main_elm);
    }
}

#[test]
fn test_attribute_graph_dispatcher() {
    let mut graph = AttributeGraph::from(0);

    let test_messages = r#"
    ```
    add test_attr             .text testing text attr
    add test_attr_empty       .EMPTY
    add test_attr_bool        .BOOL true
    add test_attr_int         .INT 510982
    add test_attr_int_pair    .INT_PAIR 5000, 1200
    add test_attr_int_range   .INT_RANGE 500, 0, 1000
    add test_attr_float       .FLOAT 510982.12
    add test_attr_float_pair  .FLOAT_PAIR 5000.0, 1200.12
    add test_attr_float_range .FLOAT_RANGE 500.0, 0.0, 1000.0
    import 10 test_attr       .TEXT this value is imported
    define test_attr node
    #
    # Note:
    # Since `new_text_attr` doesn't already exist
    # edit/commit will insert a new attribute into the graph
    # This is useful when extending the graph.
    #
    define new_text_attr node
    # Can define multiple symbols for the same attr
    define new_text_attr edit
    edit new_text_attr::node test_attr23 .TEXT adding a new text attribute
    apply new_text_attr node
    ```
    "#;

    for message in test_messages.trim().split("\n") {
        assert!(graph.dispatch_mut(message).is_ok());
    }

    assert!(graph.contains_attribute("test_attr"));
    assert!(graph.contains_attribute("test_attr_int"));
    assert!(graph.contains_attribute("test_attr_int_pair"));
    assert!(graph.contains_attribute("test_attr_int_range"));
    assert!(graph.contains_attribute("test_attr_float"));
    assert!(graph.contains_attribute("test_attr_float_pair"));
    assert!(graph.contains_attribute("test_attr_float_range"));
    assert!(graph.contains_attribute("test_attr_empty"));
    assert!(graph.contains_attribute("test_attr_bool"));
    assert!(graph.contains_attribute("test_attr::node"));
    assert!(graph.contains_attribute("new_text_attr::node"));
    assert!(!graph.contains_attribute("new_text_attr"));
    assert!(graph.contains_attribute("test_attr23"));

    // test graph state
    assert_eq!(
        Some(&Value::TextBuffer("testing text attr".to_string())),
        graph.find_attr_value("test_attr")
    );

    assert_eq!(
        Some(&Value::TextBuffer(
            "adding a new text attribute".to_string()
        )),
        graph.find_attr_value("test_attr23")
    );

    // test edit/commit symbols
    let test_messages = r#"
    ```
    #
    # Note:
    # Since test_attr already exists
    # edit/apply will overwrite the existing value
    #
    edit test_attr::node test_attr .TEXT testing apply attr
    apply test_attr node
    ```
    "#;

    for message in test_messages.trim().split("\n") {
        assert!(graph.dispatch_mut(message).is_ok());
    }

    assert!(graph.contains_attribute("test_attr"));
    assert_eq!(
        Some(&Value::TextBuffer("testing apply attr".to_string())),
        graph.find_attr_value("test_attr")
    );

    assert_eq!(
        Some(&Value::Bool(true)),
        graph.find_attr_value("test_attr_bool")
    );

    assert_eq!(
        Some(&Value::Empty),
        graph.find_attr_value("test_attr_empty")
    );

    assert_eq!(
        Some(&Value::Int(510982)),
        graph.find_attr_value("test_attr_int")
    );

    assert_eq!(
        Some(&Value::IntPair(5000, 1200)),
        graph.find_attr_value("test_attr_int_pair")
    );

    assert_eq!(
        Some(&Value::IntRange(500, 0, 1000)),
        graph.find_attr_value("test_attr_int_range")
    );

    assert_eq!(
        Some(&Value::Float(510982.12)),
        graph.find_attr_value("test_attr_float")
    );

    assert_eq!(
        Some(&Value::FloatPair(5000.0, 1200.12)),
        graph.find_attr_value("test_attr_float_pair")
    );

    assert_eq!(
        Some(&Value::FloatRange(500.0, 0.0, 1000.0)),
        graph.find_attr_value("test_attr_float_range")
    );

    assert_eq!(
        Some(&Value::Symbol("node::".to_string())),
        graph.find_attr_value("test_attr::node")
    );

    // Test find_remove
    assert!(graph.dispatch_mut("find_remove test_attr").is_ok());
    assert!(!graph.contains_attribute("test_attr"));

    // Find and validate the graph after it has been imported
    if let Some(imported) = graph.find_imported_graph(10) {
        assert!(imported.contains_attribute("test_attr"));
        assert_eq!(
            Some(&Value::TextBuffer("this value is imported".to_string())),
            imported.find_attr_value("test_attr")
        );
        println!(
            "# Imported\n {}",
            imported
                .save()
                .expect("should be able to save imported attribute graph")
        );

        assert!(graph.dispatch_mut("copy 10 test_attr").is_ok());
        assert!(graph.contains_attribute("test_attr"));
        assert_eq!(
            Some(&Value::TextBuffer("this value is imported".to_string())),
            graph.find_attr_value("test_attr")
        );
    } else {
        assert!(false, "could not find imported graph");
    }

    println!(
        "# Result\n {}",
        graph
            .save()
            .expect("should be able to save an attribute graph")
    );

    println!(
        "# Symbols\n{}",
        graph
            .find_symbols("node")
            .iter()
            .map(|a| format!("{:?}", a))
            .collect::<Vec<String>>()
            .join("\n")
    )
}

#[test]
fn test_binary_vec_parse() {
    let mut graph = AttributeGraph::from(0);
    let test_message = format!(
        "add test_bin .BINARY_VECTOR {}",
        base64::encode(b"test values")
    );

    println!("{}", test_message);
    assert!(graph.dispatch_mut(test_message).is_ok());

    if let Some(Value::BinaryVector(test_bin)) = graph.find_attr_value("test_bin") {
        if let Some(test) = base64::decode(test_bin)
            .ok()
            .and_then(|t| String::from_utf8(t).ok())
        {
            assert_eq!(test, "test values".to_string());
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

#[derive(Logos, Debug, Hash, Clone, PartialEq, PartialOrd)]
pub enum AttributeGraphEvents {
    /// Usage: add {`attribute-name`} {`value-type`} {`remaining as value`}
    /// Example: add test_attr .TEXT remaining text is text
    /// Adds a new attribute to the graph. Types omitted from this event are symbol, reference, and binary-vector
    #[token("add")]
    Add,
    /// Usage: find_remove {`attribute-name`}
    /// Example: find_remove test_attr
    /// Finds and removes an attribute from the graph by attribute-name
    #[token("find_remove")]
    FindRemove,
    /// Usage: import {`external entity id`} {`attribute-name`} {`value-type token`} {`remaining is parsed corresponding to value-type token`}
    /// Example: import 10 test_attr .TEXT remaining text is text
    /// Imports an attribute to the graph, Types omitted from this event are symbol, reference, and binary-vector
    #[token("import")]
    Import,
    /// Usage: copy {`external entity id`} {`attribute-name`}
    /// Examples: copy 10 test_attr
    ///           copy 10
    /// Copies imported attribute/s to the graph. Types omitted from this event are symbol, reference, and binary-vector
    /// Implementation requires that attributes are imported to the graph before copy message is handled
    #[token("copy")]
    Copy,
    /// Usage: define {`attribute-name`} {`symbol-name`}
    /// Examples: define test_attr node
    /// Defines and adds a symbol for a specified attribute name
    /// If the attribute doesn't already exist, it is not added.
    /// The format of the name for the symbol attribute is {`attribute-name`}::{`symbol-name`}
    /// The value of the symbol will be {`symbol-name`}::
    #[token("define")]
    Define,
    /// Usage: apply {`attribute-name`} {`symbol-name`}
    /// Examples: apply test_attr node
    /// If a symbol has been defined for attribute, and the symbol attribute has a transient value,
    /// apply will override the value with the transient value. If the attribute doesn't already exist it is added.
    /// For example if some symbol called node is defined for test_attr. Then an attribute will exist with the name test_attr:node.
    /// If some system edits the value of test_attr::node, then a transient value will exist for test_attr::node.
    /// Dispatching apply will take the transient value and write to test_attr.
    #[token("apply")]
    Apply,
    /// Usage: edit {`attribute-name`} {`new-attribute-name`} {`new-value-type`} {`remaining as value`}
    /// Examples: edit test_attr test_attr2 .TEXT editing the value of test_attr
    /// Set's the transient value for an attribute. Types omitted from this event are symbol, reference, and binary-vector.
    #[token("edit")]
    Edit,
    #[token("from")]
    From,
    #[token("to")]
    To,
    #[token("publish")]
    Publish,
    /// Usage: # Here is a helpful comment
    #[token("#")]
    Comment,
    /// Usage:
    /// ```
    /// add test_attr .TEXT remaining text is the value
    /// ```
    #[token("```")]
    BlockDelimitter,
    // Logos requires one token variant to handle errors,
    // it can be named anything you wish.
    #[error]
    // We can also use this variant to define whitespace,
    // or any other matches we wish to skip.
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

/// Elements contained within an attribute graph
#[derive(Logos, Debug, Hash, Clone, PartialEq, PartialOrd)]
pub enum AttributeGraphElements {
    /// text element parses all remaining text after .TEXT as a string
    #[token(".text", graph_lexer::from_text)]
    #[token(".TEXT", graph_lexer::from_text)]
    Text(Value),
    /// bool element parses remaining as bool
    #[token(".bool", graph_lexer::from_bool)]
    #[token(".BOOL", graph_lexer::from_bool)]
    Bool(Value),
    /// int element parses remaining as i32
    #[token(".int", graph_lexer::from_int)]
    #[token(".INT", graph_lexer::from_int)]
    Int(Value),
    /// int pair element parses remaining as 2 comma-delimmited i32's
    #[token(".int2", graph_lexer::from_int_pair)]
    #[token(".INT_PAIR", graph_lexer::from_int_pair)]
    IntPair(Value),
    /// int range element parses remaining as 3 comma-delimitted i32's
    #[token(".int3", graph_lexer::from_int_range)]
    #[token(".INT_RANGE", graph_lexer::from_int_range)]
    IntRange(Value),
    /// float element parses remaining as f32
    #[token(".FLOAT", graph_lexer::from_float)]
    Float(Value),
    /// float pair element parses reamining as 2 comma delimitted f32's
    #[token(".FLOAT_PAIR", graph_lexer::from_float_pair)]
    FloatPair(Value),
    /// float range element parses remaining as 3 comma delimitted f32's
    #[token(".FLOAT_RANGE", graph_lexer::from_float_range)]
    FloatRange(Value),
    /// binary vector element parses remaining as 3 comma delimitted f32's
    #[token(".BINARY_VECTOR", graph_lexer::from_binary_vector_base64)]
    BinaryVector(Value),
    /// empty element parses
    #[token(".EMPTY")]
    Empty,
    /// entity ids should be parsed before symbols
    #[regex("[0-9]+", priority = 3, callback = graph_lexer::from_entity)]
    Entity(u32),
    /// symbols must start with a character, and is composed of lowercase characters, digits, underscores, and colons
    #[regex("#[A-Za-z_.0-9]+", graph_lexer::from_string)]
    Name(String),
    /// symbols must start with a character, and is composed of lowercase characters, digits, underscores, and colons
    #[regex("[a-z]+[a-z_:0-9]*", graph_lexer::from_string)]
    Symbol(String),
    // Logos requires one token variant to handle errors,
    // it can be named anything you wish.
    #[error]
    // We can also use this variant to define whitespace,
    // or any other matches we wish to skip.
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

mod graph_lexer {
    use std::str::FromStr;

    use atlier::system::Value;
    use logos::Lexer;

    use super::AttributeGraphElements;

    pub fn from_entity(lexer: &mut Lexer<AttributeGraphElements>) -> Option<u32> {
        lexer.slice().parse().ok()
    }

    pub fn from_string(lexer: &mut Lexer<AttributeGraphElements>) -> Option<String> {
        let mut slice = lexer.slice();
        if slice.starts_with('#') {
            slice = &slice[1..];
        }

        Some(slice.to_string())
    }

    pub fn from_text(lexer: &mut Lexer<AttributeGraphElements>) -> Option<Value> {
        let remaining = lexer.remainder().trim().to_string();

        Some(Value::TextBuffer(remaining))
    }

    pub fn from_bool(lexer: &mut Lexer<AttributeGraphElements>) -> Option<Value> {
        if let Some(value) = lexer.remainder().trim().parse().ok() {
            Some(Value::Bool(value))
        } else {
            None
        }
    }

    pub fn from_int(lexer: &mut Lexer<AttributeGraphElements>) -> Option<Value> {
        if let Some(value) = lexer.remainder().trim().parse::<i32>().ok() {
            Some(Value::Int(value))
        } else {
            None
        }
    }

    pub fn from_int_pair(lexer: &mut Lexer<AttributeGraphElements>) -> Option<Value> {
        let pair = from_comma_sep::<i32>(lexer);

        match (pair.get(0), pair.get(1)) {
            (Some(f0), Some(f1)) => Some(Value::IntPair(*f0, *f1)),
            _ => None,
        }
    }

    pub fn from_int_range(lexer: &mut Lexer<AttributeGraphElements>) -> Option<Value> {
        let range = from_comma_sep::<i32>(lexer);

        match (range.get(0), range.get(1), range.get(2)) {
            (Some(f0), Some(f1), Some(f2)) => Some(Value::IntRange(*f0, *f1, *f2)),
            _ => None,
        }
    }

    pub fn from_float(lexer: &mut Lexer<AttributeGraphElements>) -> Option<Value> {
        if let Some(value) = lexer.remainder().trim().parse::<f32>().ok() {
            Some(Value::Float(value))
        } else {
            None
        }
    }

    pub fn from_float_pair(lexer: &mut Lexer<AttributeGraphElements>) -> Option<Value> {
        let pair = from_comma_sep::<f32>(lexer);
        match (pair.get(0), pair.get(1)) {
            (Some(f0), Some(f1)) => Some(Value::FloatPair(*f0, *f1)),
            _ => None,
        }
    }

    pub fn from_float_range(lexer: &mut Lexer<AttributeGraphElements>) -> Option<Value> {
        let range = from_comma_sep::<f32>(lexer);

        match (range.get(0), range.get(1), range.get(2)) {
            (Some(f0), Some(f1), Some(f2)) => Some(Value::FloatRange(*f0, *f1, *f2)),
            _ => None,
        }
    }

    pub fn from_binary_vector_base64(lexer: &mut Lexer<AttributeGraphElements>) -> Option<Value> {
        match base64::decode(lexer.remainder().trim()) {
            Ok(content) => Some(Value::BinaryVector(content)),
            Err(_) => None,
        }
    }

    fn from_comma_sep<T>(lexer: &mut Lexer<AttributeGraphElements>) -> Vec<T>
    where
        T: FromStr,
    {
        lexer
            .remainder()
            .trim()
            .split(",")
            .filter_map(|i| i.trim().parse().ok())
            .collect()
    }
}
