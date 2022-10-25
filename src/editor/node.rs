use std::sync::Arc;

use atlier::system::App;

use crate::prelude::{Connection, Cursor};

use super::Appendix;

/// Struct for visualizing entities w/ a cursor and/or connection component,
/// 
#[derive(Hash, PartialEq, Eq)]
pub struct Node {
    /// The Cursor component stores entities this node points to
    /// 
    pub cursor: Option<Cursor>,
    /// The conenction component stores entities that point to this node, 
    /// 
    pub connection: Option<Connection>,
    /// Reference to an appendix, to lookup desciriptions on entities this node references,
    /// 
    pub appendix: Arc<Appendix>
}

impl App for Node {
    fn name() -> &'static str {
        "node"
    }

    fn edit_ui(&mut self, ui: &imgui::Ui) {
        todo!()
    }

    fn display_ui(&self, ui: &imgui::Ui) {
        if let Some(connection) = self.connection.as_ref() {
            if let Some(name) = self.appendix.name(&connection.entity()) {
                ui.label_text("name", name);
            }
        }
    }
}
