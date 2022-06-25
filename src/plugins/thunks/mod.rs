
use crate::AttributeGraph;
use atlier::system::Attribute;
use imgui::Ui;
use specs::{storage::DenseVecStorage, Entity};
use specs::Component;

mod println;
pub use println::Println;

mod write_files;
use tokio::{runtime::Handle, task::JoinHandle, sync::mpsc::Sender};
pub use write_files::WriteFiles;

pub mod demo {
    use super::write_files::demo;
    pub use demo::WriteFilesDemo;
}

use super::{BlockContext, Plugin};

/// Thunk is a function that can be passed around for the system to call later
#[derive(Component, Clone)]
#[storage(DenseVecStorage)]
pub struct Thunk(pub &'static str, pub fn(&mut ThunkContext) -> Option<JoinHandle<()>>);

impl Thunk {
    pub fn from_plugin<P>() -> Self
    where
        P: Plugin<ThunkContext>,
    {
        Self(P::symbol(), P::call_with_context)
    }

    pub async fn start(&self, context: &mut ThunkContext, handle: Handle) {
        let Thunk(symbol, thunk) = self;

        context.handle = Some(handle.clone());

        if let Some(join_handle) = thunk(context) {
            match join_handle.await {
                Ok(_) => {
                    context.block.update_block("thunk", |t| {
                        t.add_text_attr("thunk_symbol", symbol.to_string());
                    });
                },
                Err(err) => {
                    context.block.update_block("thunk", |t| {
                        t.add_text_attr("error", format!("error {}", err));
                    });
                },
            }
        }
    }

    pub fn show(&self, context: &mut ThunkContext, ui: &Ui) {
        ui.set_next_item_width(130.0);
        if ui.button(context.label(self.0)) {
            let Thunk(.., thunk) = self;
            thunk(context);
        }
    }
}


pub type StatusUpdate = (Entity, f32, String);

/// ThunkContext provides common methods for updating the underlying state graph,
/// in the context of a thunk.
#[derive(Component, Default, Clone)]
#[storage(DenseVecStorage)]
pub struct ThunkContext { 
    pub block: BlockContext,
    handle: Option<Handle>,
    status_updates: Option<Sender<StatusUpdate>>
}

impl From<AttributeGraph> for ThunkContext {
    fn from(g: AttributeGraph) -> Self {
        Self {
            block: BlockContext::from(g),
            handle: None,
            status_updates: None,
        }
    }
}

impl AsRef<AttributeGraph> for ThunkContext {
    fn as_ref(&self) -> &AttributeGraph {
        self.block.as_ref()
    }
}

impl AsMut<AttributeGraph> for ThunkContext {
    fn as_mut(&mut self) -> &mut AttributeGraph {
        self.block.as_mut()
    }
}

impl ThunkContext {
    pub fn handle(&self) -> Option<Handle> {
        self.handle.as_ref().and_then(|h| Some(h.clone()))
    }

    pub fn symbol(&self) -> Option<String> {
        if let Some(thunk) = self.block.get_block("thunk") {
            thunk.find_text("thunk_symbol")
        } else {
            None
        }
    }

    /// Updates error block
    pub fn error(&mut self, record: impl FnOnce(&mut AttributeGraph)) {
        self.block.update_block("error", record);
    }

    /// Update publish block
    pub fn publish(&mut self, update: impl FnOnce(&mut AttributeGraph)) {
        self.block.update_block("publish", update);
    }

    /// Receives values from the accept block, and updates the destination block with the new values
    pub fn accept(&mut self, dest_block: impl AsRef<str>, accept: impl Fn(&Attribute) -> bool) {
        if let Some(accept_block) = self.block.get_block("accept") {
            for (name, value) in accept_block
                .iter_attributes()
                .filter(|a| accept(a))
                .map(|a| (a.name(), a.value()))
            {
                self.block.update_block(dest_block.as_ref(), |u| {
                    u.with(name, value.clone());
                });
            }
        }
    }

    pub fn label(&self, label: impl AsRef<str>) -> impl AsRef<str> {
        format!("{} {:#2x}", label.as_ref(), self.as_ref().hash_code() as u16 )
    }
}
