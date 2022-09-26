use std::collections::BTreeSet;

use specs::prelude::*;
use atlier::system::Value;
use tracing::{event, Level};

/// A catalog is used to store and retrieve a collection of items. Typically data is retrieved using human-friendly
/// concepts, 
///     such as tagging features or categories relevant to the data, 
///     physical states such as size or age, 
///     relationships w/ other data in the catalog, 
/// This process is refferred to as a "look-up" or "search", and most commonly a "query".
/// 
/// A catalog reader is a system data type for systems that need to read items
/// 
#[derive(SystemData)]
pub struct CatalogReader<'a, I> 
where
    I: Item + Component
{
    pub entities: Entities<'a>,
    pub items: ReadStorage<'a, I>,
}

/// A catalog writer is a system data type for indexing, and writing
/// items to world storage.
/// 
#[derive(SystemData)]
pub struct CatalogWriter<'a, I> 
where
    I: Item + Component
{
    pub entities: Entities<'a>,
    pub items: WriteStorage<'a, I>,
}

/// The intention of this trait is to combine indexing / deserialization semantics into a single trait using the 
/// Visitor pattern.
/// Serde traits could have been used instead, but after examination seemed a bit overkill for the needs of this
/// trait. That being said, an implementation based on serde would allow more interop options, and make the runtime more flexible
/// as a whole.
/// 
/// Default visit methods emit a WARN level event, to track cases where an attribute value is deserialized
/// but not handled by the implementing type.
/// 
/// By implementing a visit method, the implementing type is responsible for validating and interpreting attributes to set it's own state, and doesn't need
/// to concern itself w/ storage of attributes.
/// 
pub trait Item {
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
        event!(Level::WARN, "visit_bool not implemented")
    }

    /// Visits self w/ a name and int 
    /// 
    fn visit_int(&mut self, _name: impl AsRef<str>, _value: i32) {
        event!(Level::WARN, "visit_int not implemented")
    }

    /// Visits self w/ a name and int_pair
    /// 
    fn visit_int_pair(&mut self, _name: impl AsRef<str>, _value: [i32; 2]) {
        event!(Level::WARN, "visit_int_pair not implemented")
    }

    /// Visits self w/ a name and int_range
    /// 
    fn visit_int_range(&mut self, _name: impl AsRef<str>, _value: [i32; 3]) {
        event!(Level::WARN, "visit_int_range not implemented")
    }

    /// Visits self w/ a name and float
    /// 
    fn visit_float(&mut self, _name: impl AsRef<str>, _value: f32) {
        event!(Level::WARN, "visit_float not implemented")
    }

    /// Visits self w/ a name and a float pair
    /// 
    fn visit_float_pair(&mut self, _name: impl AsRef<str>, _value: [f32; 2]) {
        event!(Level::WARN, "visit_float_pair not implemented")
    }

    /// Visits self w/ a name and float range
    /// 
    fn visit_float_range(&mut self, _name: impl AsRef<str>, _value: [f32; 3]) {
        event!(Level::WARN, "visit_float_range not implemented")
    }

    /// Visits self w/ a name and binary vector
    /// 
    fn visit_binary_vec(&mut self, _name: impl AsRef<str>, _value: impl Into<Vec<u8>>) {
        event!(Level::WARN, "visit_binary_vec not implemented")
    }

    /// Visits self w/ a name and symbol
    /// 
    fn visit_symbol(&mut self, _name: impl AsRef<str>, _value: impl AsRef<str>) {
        event!(Level::WARN, "visit_symbol not implemented")
    }

    /// Visits self w/ a name and text
    /// 
    fn visit_text(&mut self, _name: impl AsRef<str>, _value: impl AsRef<str>) {
        event!(Level::WARN, "visit_text not implemented")
    }

    /// Visits self w/ a name and value
    /// 
    fn visit_reference(&mut self, _name: impl AsRef<str>, _value: u64) {
        event!(Level::WARN, "visit_reference not implemented")
    }

    fn visit_complex(&mut self, _name: impl AsRef<str>, _value: impl Into<BTreeSet<String>>) {
        event!(Level::WARN, "visit_complex not implemented")
    }

    /// Visits self w/ a name and value and calls the corresponding visit method
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
            Value::Reference(r) => self.visit_reference(name, *r),
            Value::Complex(complex) => self.visit_complex(name, complex.clone()),
            Value::Empty => unimplemented!("empty value is not implemented"),
        }
    }
}