use runmd::prelude::*;

use crate::{runtime::Runtime, plugins::ThunkContext};


impl NodeProvider for Runtime {
    fn provide(&self, name: &str, tag: Option<&str>, input: Option<&str>) -> Option<BoxedNode> {
        todo!()
    }
}

#[runmd::prelude::async_trait]
impl runmd::prelude::ExtensionLoader for ThunkContext {
    async fn load_extension(&self, extension: &str, input: Option<&str>) -> Option<BoxedNode> {
        match extension {
            "application/repo.lifec.v1.plugin" => {
                
            },
            _ => {}
        }

        None
    }
}

impl Node for ThunkContext {
    fn set_info(&mut self, node_info: NodeInfo, block_info: BlockInfo) {
        todo!()
    }

    fn define_property(&mut self, name: &str, tag: Option<&str>, input: Option<&str>) {
        todo!()
    }

    fn completed(self: Box<Self>) {
        todo!()
    }
}