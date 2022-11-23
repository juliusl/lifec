use atlier::system::Value;
use reality::SpecialAttribute;

/// Special attribute to enable a local address for a udp socket,
/// 
pub struct UDP; 

impl SpecialAttribute for UDP {
    fn ident() -> &'static str {
        "udp"
    }

    fn parse(parser: &mut reality::AttributeParser, content: impl AsRef<str>) {
        if content.as_ref().is_empty() {
            parser.define("udp", "127.0.0.1:0");
        } else {
            parser.define("udp", Value::Symbol(content.as_ref().to_string()));
        }
    }
}