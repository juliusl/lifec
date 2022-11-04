use specs::{Component, DefaultVecStorage};

mod chaos;
pub use chaos::Chaos;

mod test_host;
pub use test_host::TestHost;

mod test_host_sender;
pub use test_host_sender::TestHostSender;

pub struct Test;

/// Debug component 
/// 
#[derive(Component, Default)]
#[storage(DefaultVecStorage)]
pub struct Debug();

