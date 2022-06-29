
use specs::Component;
use specs::storage::DenseVecStorage;

use super::StartButton;

/// This component is to enable sequencing within a task
#[derive(Component, Clone, Default)]
#[storage(DenseVecStorage)]
pub struct NextButton(
    /// Owner
    pub StartButton,
    /// Next
    pub Option<StartButton>
);

