
mod listener;
pub use listener::PluginListener;

mod broker;
pub use broker::Broker as PluginBroker;

mod features;
pub use features::Features as PluginFeatures;

use specs::SystemData;
use specs::prelude::*;

/// System data for plugins,
///
#[derive(SystemData)]
pub struct Plugins<'a> { 
    features: PluginFeatures<'a>,
}

impl<'a> Plugins<'a> {
    /// Returns a reference to plugin features,
    /// 
    pub fn features(&self) -> &PluginFeatures<'a> {
        let Plugins { features, .. } = self;

        features
    }
}
