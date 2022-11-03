use specs::{Component, HashMapStorage};

/// Component to indicate a limit,
/// 
#[derive(Debug, Clone, Component)]
#[storage(HashMapStorage)]
pub struct Limit(pub usize);

impl Limit {
    /// Takes one from the limit, returns true if one was taken
    /// 
    pub fn take_one(&mut self) -> bool {
        if self.0 > 0 {
            self.0 -= 1;
            true
        } else {
            false
        }
    }
}