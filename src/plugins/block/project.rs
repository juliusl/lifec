
use std::collections::BTreeMap;

use reality::Block;
use specs::{Component, Entity};
use specs::storage::HashMapStorage;

#[derive(Debug, Default, Component, Clone)]
#[storage(HashMapStorage)]
pub struct Project {
    blocks: BTreeMap<String, Entity>,

}

impl Project {

}
