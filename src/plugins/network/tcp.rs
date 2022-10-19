use atlier::system::Value;
use reality::SpecialAttribute;

/// Special attribute to enable a local address for a tcp listener,
/// 
pub struct TCP; 

impl SpecialAttribute for TCP {
    fn ident() -> &'static str {
        "tcp"
    }

    fn parse(parser: &mut reality::AttributeParser, content: impl AsRef<str>) {
        if content.as_ref().is_empty() {
            parser.define("tcp", "127.0.0.1:0");
        } else {
            parser.define("tcp", Value::Symbol(content.as_ref().to_string()));
        }
    }
}