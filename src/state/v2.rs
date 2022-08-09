mod attribute_index;
pub use attribute_index::AttributeIndex;

mod query;
pub use query::Query;

mod tests {
    use crate::catalog::Item;

    #[test]
    fn test_query() {
        use crate::{AttributeGraph, plugins::ThunkContext, state::AttributeIndex};

        // Test simple case where the src is just a thunk context
        let src = AttributeGraph::from(0)
            .with_text("name", "bob")
            .with_int("age", 99)
            .with_bool("is_alias", true)
            .with_binary("test_bin", vec![b'h', b'e', b'l', b'l', b'o'])
            .with_float_range("test_float_range", &[3.1, 1.4, 4.5])
            .with_float_pair("test_float_pair", &[3.1, 1.4])
            .with_float("test_float", 3.1)
            .with_int_pair("test_int_pair", &[3, 1])
            .with_int_range("test_int_range", &[3, 1, 4])
            .with_symbol("test_symbol", "cool_symbol")
            .to_owned();
        let src = ThunkContext::from(src);
        
        let query = src.query();
        let query = query
            .find_text("name")
            .find_int("age")
            .find_bool("is_alias")
            .find_binary("test_bin")
            .find_float_range("test_float_range")
            .find_float_pair("test_float_pair")
            .find_float("test_float")
            .find_int_pair("test_int_pair")
            .find_int_range("test_int_range")
            .find_symbol("test_symbol");
    
        let mut person = Person::default();
        query.evaluate(&mut person);

        assert_eq!(person.name, "bob");
        assert_eq!(person.age, 99);
        assert_eq!(person.is_alias, true);
        assert_eq!(person.test_bin, vec![b'h', b'e', b'l', b'l', b'o']);
        assert_eq!(person.test_float_range, [3.1, 1.4, 4.5]);
        assert_eq!(person.test_float_pair, [3.1, 1.4]);
        assert_eq!(person.test_float, 3.1);
        assert_eq!(person.test_int_pair, [3, 1]);
        assert_eq!(person.test_int_range, [3, 1, 4]);
        assert_eq!(person.test_symbol, "cool_symbol");
        eprintln!("{:#?}", person);
        
        let cached = &mut query.cache();

        let mut person_from_cached = Person::default();
        cached.cached(&mut person_from_cached);

        let person = person_from_cached;
        assert_eq!(person.name, "bob");
        assert_eq!(person.age, 99);
        assert_eq!(person.is_alias, true);
        assert_eq!(person.test_bin, vec![b'h', b'e', b'l', b'l', b'o']);
        assert_eq!(person.test_float_range, [3.1, 1.4, 4.5]);
        assert_eq!(person.test_float_pair, [3.1, 1.4]);
        assert_eq!(person.test_float, 3.1);
        assert_eq!(person.test_int_pair, [3, 1]);
        assert_eq!(person.test_int_range, [3, 1, 4]);
        assert_eq!(person.test_symbol, "cool_symbol");
        eprintln!("{:#?}", person);
    }

    #[derive(Debug, Default)]
    struct Person {
        name: String,
        age: u32,
        is_alias: bool,
        test_bin: Vec<u8>,
        test_float: f32,
        test_float_pair: [f32; 2],
        test_float_range: [f32; 3],
        test_symbol: String,
        test_int_pair: [i32; 2],
        test_int_range: [i32; 3],
    }

    /// TODO: add a proc macro that derives this
    /// TODO: could try reusing serde traits
    impl Item for Person {
        fn visit_text(&mut self, _name: impl AsRef<str>, _value: impl AsRef<str>) {
            if _name.as_ref() == "name" {
                self.name = _value.as_ref().to_string();
            }
        }

        fn visit_symbol(&mut self, _name: impl AsRef<str>, _value: impl AsRef<str>) {
            if _name.as_ref() == "test_symbol" {
                self.test_symbol = _value.as_ref().to_string();
            }
        }

        fn visit_int(&mut self, _name: impl AsRef<str>, _value: i32) {
            if _name.as_ref() == "age" {
                self.age = _value as u32; 
            }
        }

        fn visit_int_range(&mut self, _name: impl AsRef<str>, _value: [i32; 3]) {
            if _name.as_ref() == "test_int_range" {
                self.test_int_range = _value; 
            }
        }

        fn visit_int_pair(&mut self, _name: impl AsRef<str>, _value: [i32; 2]) {
            if _name.as_ref() == "test_int_pair" {
                self.test_int_pair = _value; 
            }
        }

        fn visit_bool(&mut self, _name: impl AsRef<str>, _value: bool) {
            if _name.as_ref() == "is_alias" {
                self.is_alias = _value;
            }
        }

        fn visit_float_pair(&mut self, _name: impl AsRef<str>, _value: [f32; 2]) {
            if _name.as_ref() == "test_float_pair" {
                self.test_float_pair = _value;
            }
        }

        fn visit_float_range(&mut self, _name: impl AsRef<str>, _value: [f32; 3]) {
            if _name.as_ref() == "test_float_range" {
                self.test_float_range = _value;
            }
        }

        fn visit_float(&mut self, _name: impl AsRef<str>, _value: f32) {
            if _name.as_ref() == "test_float" {
                self.test_float = _value;
            }
        }

        fn visit_binary_vec(&mut self, _name: impl AsRef<str>, _value: impl Into<Vec<u8>>) {
            if _name.as_ref() == "test_bin" {
                self.test_bin = _value.into();
            }
        }
    }
}



