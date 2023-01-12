use std::io::ErrorKind;

use reality::AttributeParser;
use serde::{Deserialize, Serialize};
use specs::{Component, VecStorage};

/// Component to distinguish adhoc workspace operations,
///
#[derive(Debug, Component, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[storage(VecStorage)]
pub struct Adhoc {
    /// Name of the adhoc component
    ///
    pub name: String,
    /// Tag config,
    ///
    pub tag: String,
}

impl Adhoc {
    /// Returns an Adhoc struct deriving from an attribute parser state,
    /// 
    pub fn from_parser(name: impl AsRef<str>, symbol: impl AsRef<str>, parser: &mut AttributeParser) -> Result<Self, std::io::Error> {
        let name = name.as_ref().to_string();
        let symbol = symbol.as_ref();

        let tag = if let Some(tag) = parser.name() {
            if tag != &symbol {
                let tag = format!("{tag}.operation");
                parser.set_name(&tag);
                tag.to_string()
            } else {
                String::from("operation")
            }
        } else {
            return Err(std::io::Error::new(ErrorKind::InvalidInput, "parser missing name"));
        };

        Ok(Adhoc { name, tag })
    }

    /// Returns the tag name,
    ///
    pub fn tag(&self) -> impl AsRef<str> + '_ {
        self.tag.trim_end_matches("operation").trim_end_matches(".")
    }

    /// Returns the name of the adhoc
    ///
    pub fn name(&self) -> impl AsRef<str> + '_ {
        &self.name
    }
}
