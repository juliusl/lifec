use reality::Value;
use reality::SpecialAttribute;

use crate::{prelude::ThunkContext, state::AttributeIndex};

pub struct Fmt; 

impl Fmt {
    /// Applies any fmt symbol values to a target string,
    /// 
    pub fn apply(tc: &ThunkContext, target: impl AsRef<str>) -> String {
        let mut target = target.as_ref().to_string();
        for fmt in tc.state().find_symbol_values("fmt") {
            // Search the previous state
            if let Some(value) = tc.search().find_symbol(&fmt) {
                target = target.replace(&format!("{{{fmt}}}"), value.as_str());
            }
        }
        
        target
    }
}

impl SpecialAttribute for Fmt {
    fn ident() -> &'static str {
        "fmt"
    }

    fn parse(parser: &mut reality::AttributeParser, content: impl AsRef<str>) {
        let entity = parser.last_child_entity().expect("should have a child entity");

        for ident in Self::parse_idents(content) { 
            parser.define_child(entity, "fmt", Value::Symbol(ident));
        }
    }
}