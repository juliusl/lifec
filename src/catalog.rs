use std::{collections::BTreeMap, fmt::Debug};

use atlier::system::Value;
use specs::{System, Entity, ReadStorage, Entities, Join, WriteStorage};
use tokio::sync::mpsc;
use tracing::{event, Level};

use crate::{AttributeGraph, Runtime, plugins::ThunkContext};

/// A catalog is a service to store and retrieve data. Typically data is retrieved using human-friendly
/// concepts, 
///     such as tagging features or categories relevant to the data, 
///     physical states such as size or age, 
///     relationships w/ other data in the catalog, 
/// This process is refferred to as a "look-up" or "search", and most commonly a "query".
/// 
/// The process of storing this data is typically called "indexing". 
/// 
/// This system provides this functionality to entities, that need it at runtime. The aim
/// is not to reinvent the actual indexing/querying capabilities of already existing db formats.
/// Instead this system should merely bootstrap db's to the entities in a way that the implementation is opaque 
/// to users. Since lifec plugins only use attributes for data, config, bootstrapping in this context means to 
/// read/write the backing db w/ attribute data.
/// 
pub struct Catalog {
    /// Receivers, 
    receivers: BTreeMap<Entity, mpsc::Receiver<AttributeGraph>>
}

impl<'a> System<'a> for Catalog {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Runtime>,
        WriteStorage<'a, ThunkContext>,
    );

    fn run(&mut self, (entities, runtimes, _thunk_contexts): Self::SystemData) {
        for (entity, _runtime) in (&entities, &runtimes).join() {
            match self.receivers.get(&entity) {
                Some(_rx) => {
                    todo!()
                },
                None => {
                    
                },
            }
        }
    }
}

/// The intention of this trait is to combine indexing / deserialization semantics into a single trait
/// Serde traits could have been used instead, but after examination seemed a bit overkill for the needs of this
/// trait. That being said, an implementation based on serde would allow more interop options, and make the runtime more flexible
/// as a whole.
/// 
/// Default visit methods emit a WARN level event, to track cases where an attribute value is deserialized
/// but not handled by the implementing type.
/// 
/// By implementing a visit method, the implementing type is responsible for validating and setting it's own state, and doesn't need
/// to concern itself w/ storage of attributes.
/// 
pub trait Item 
where
    Self: Debug
{
    /// Returns a reference to implementing type
    /// 
    fn item_ref(&self) -> &Self {
        self
    }

    /// Returns a mutable reference to implementing type
    /// 
    fn item_mut(&mut self) -> &mut Self {
        self 
    }

    /// Visits self w/ a name and bool 
    /// 
    fn visit_bool(&mut self, _name: impl AsRef<str>, _value: bool) {
        event!(Level::WARN, "visit_bool not implemented {:#?}", self)
    }

    /// Visits self w/ a name and int 
    /// 
    fn visit_int(&mut self, _name: impl AsRef<str>, _value: i32) {
        event!(Level::WARN, "visit_int not implemented {:#?}", self)
    }

    /// Visits self w/ a name and int_pair
    /// 
    fn visit_int_pair(&mut self, _name: impl AsRef<str>, _value: [i32; 2]) {
        event!(Level::WARN, "visit_int_pair not implemented {:#?}", self)
    }

    /// Visits self w/ a name and int_range
    /// 
    fn visit_int_range(&mut self, _name: impl AsRef<str>, _value: [i32; 3]) {
        event!(Level::WARN, "visit_int_range not implemented {:#?}", self)
    }

    /// Visits self w/ a name and float
    /// 
    fn visit_float(&mut self, _name: impl AsRef<str>, _value: f32) {
        event!(Level::WARN, "visit_float not implemented {:#?}", self)
    }

    /// Visits self w/ a name and a float pair
    /// 
    fn visit_float_pair(&mut self, _name: impl AsRef<str>, _value: [f32; 2]) {
        event!(Level::WARN, "visit_float_pair not implemented {:#?}", self)
    }

    /// Visits self w/ a name and float range
    /// 
    fn visit_float_range(&mut self, _name: impl AsRef<str>, _value: [f32; 3]) {
        event!(Level::WARN, "visit_float_range not implemented {:#?}", self)
    }

    /// Visits self w/ a name and binary vector
    /// 
    fn visit_binary_vec(&mut self, _name: impl AsRef<str>, _value: impl Into<Vec<u8>>) {
        event!(Level::WARN, "visit_binary_vec not implemented {:#?}", self)
    }

    /// Visits self w/ a name and symbol
    /// 
    fn visit_symbol(&mut self, _name: impl AsRef<str>, _value: impl AsRef<str>) {
        event!(Level::WARN, "visit_symbol not implemented {:#?}", self)
    }

    /// Visits self w/ a name and text
    /// 
    fn visit_text(&mut self, _name: impl AsRef<str>, _value: impl AsRef<str>) {
        event!(Level::WARN, "visit_text not implemented {:#?}", self)
    }

    /// Visits self w/ a name and value and calls the corresponding visit api.
    /// 
    fn visit(&mut self, name: impl AsRef<str>, value: &Value) {
        match value {
            Value::Bool(b) => self.visit_bool(name, *b),
            Value::TextBuffer(t) => self.visit_text(name, t),
            Value::Int(i) => self.visit_int(name, *i),
            Value::IntPair(i0, i1) => self.visit_int_pair(name, [*i0, *i1]),
            Value::IntRange(i0, i1, i2) => self.visit_int_range(name, [*i0, *i1, *i2]),
            Value::Float(f) => self.visit_float(name, *f),
            Value::FloatPair(f0, f1) => self.visit_float_pair(name, [*f0, *f1]),
            Value::FloatRange(f0, f1, f2) => self.visit_float_range(name, [*f0, *f1, *f2]),
            Value::BinaryVector(v) => self.visit_binary_vec(name, v.to_vec()),
            Value::Symbol(s) => self.visit_symbol(name, s),
            Value::Reference(_) => unimplemented!("reference value is not implemented"),
            Value::Empty => unimplemented!("empty value is not implemented"),
        }
    }
}