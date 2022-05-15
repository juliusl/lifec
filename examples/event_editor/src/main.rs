
use std::fmt::Display;

use lifec::{editor::{start_runtime_editor}, RuntimeState};
use specs::{Component, DenseVecStorage};

fn main() {
    start_runtime_editor::<Test>();
}

#[derive(Default, Clone)]
struct Test {

}

pub struct TestRuntimeError {

}

impl Display for Test {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "this is a test")
    }
}

impl RuntimeState for Test {
    type Error = TestRuntimeError;

    type State = Self;

    fn load<S: AsRef<str> + ?Sized>(&self, init: &S) -> Self
    where
        Self: Sized {
        self.clone()
    }

    fn process<S: AsRef<str> + ?Sized>(&self, msg: &S) -> Result<Self::State, Self::Error> {
        Ok(self.clone())
    }
}

impl Component for Test {
    type Storage = DenseVecStorage<Self>;
}
