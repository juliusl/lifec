use std::{any::Any, fmt::Display};

use atlier::system::App;
use specs::{Component, HashMapStorage};

use super::Edit;

#[derive(Clone)]
pub struct Section<S: Any + Send + Sync + Clone> {
    pub title: String,
    pub editor: Edit<S>,
    pub state: S,
}

impl<S: Any + Send + Sync + Clone> Component for Section<S> {
    type Storage = HashMapStorage<Self>;
}

impl<S: Any + Send + Sync + Clone + App + Display> From<S> for Section<S> {
    fn from(initial: S) -> Self {
        Section {
            title: format!("{}: {}", S::title().to_string(), initial),
            editor: Edit(S::show_editor),
            state: initial,
        }
    }
}
