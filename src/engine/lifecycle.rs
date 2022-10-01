use super::{Fork, Next, Repeat, Loop};
use crate::{Engine, Exit};
use specs::{prelude::*, Component};
use std::collections::HashMap;

/// System data for resolving lifecycle options for engines,
///
/// These components are inserted after blocks have been interpreted.
///
#[derive(SystemData)]
pub struct LifecycleResolver<'a> {
    entities: Entities<'a>,
    engine: ReadStorage<'a, Engine>,
    repeat: ReadStorage<'a, Repeat>,
    r#loop: ReadStorage<'a, Loop>,
    exit: ReadStorage<'a, Exit>,
    next: ReadStorage<'a, Next>,
    fork: ReadStorage<'a, Fork>,
    lifecycle_option: WriteStorage<'a, LifecycleOptions>,
}

impl<'a> LifecycleResolver<'a> {
    /// Returns a hash map of engines and their lifecycle options,
    /// 
    /// Also inserts a lifecycle option for each engine entity in storage.
    ///
    pub fn resolve_lifecycle(mut self) -> HashMap<Entity, LifecycleOptions> {
        let mut lifecycle_settings = HashMap::default();

        for (entity, _, repeat, r#loop, exit, next, fork) in (
            &self.entities,
            &self.engine,
            self.repeat.maybe(),
            self.r#loop.maybe(),
            self.exit.maybe(),
            self.next.maybe(),
            self.fork.maybe(),
        )
            .join()
        {
            let option = match (repeat, r#loop, exit, next, fork) {
                (Some(Repeat(Some(remaining))), None, None, None, None) if *remaining > 0 => {
                    LifecycleOptions::Repeat {
                        remaining: *remaining,
                        start: entity,
                    }
                }
                (None, Some(Loop), None, None, None) => LifecycleOptions::Loop(entity),
                (None, None, Some(Exit(Some(()))), None, None) => LifecycleOptions::Exit(Some(())),
                (None, None, None, Some(Next(Some(next))), None) => LifecycleOptions::Next(*next),
                (None, None, None, None, Some(Fork(forks))) => LifecycleOptions::Fork(forks.to_vec()),
                _ => LifecycleOptions::Exit(Some(())),
            };

            lifecycle_settings.insert(entity, option.clone());

            self.lifecycle_option
                .insert(entity, option.clone())
                .expect("should be able to insert");
        }

        lifecycle_settings
    }
}

/// Enumeration of lifecycle options for engines that have completed,
///
#[derive(Debug, Component, Clone)]
#[storage(DenseVecStorage)]
pub enum LifecycleOptions {
    /// Repeat the engine,
    ///
    Repeat { remaining: usize, start: Entity },
    /// Signal multiple engines to begin,
    ///
    Fork(Vec<Entity>),
    /// Signal a single entity to begin,
    ///
    Next(Entity),
    /// Loop indefinitely,
    ///
    Loop(Entity),
    /// (Default) Signals that this engine should exit next,
    ///
    /// All engines must have signaled for exit,
    ///
    /// If any engines remain, such as Loop or Repeat,
    /// then the Host will not exit from should_exit().
    ///
    /// Repeat will eventually resolve to Exit,
    ///
    Exit(Option<()>),
    /// Will execute once and exit
    /// 
    Once,
}

impl LifecycleOptions {
    /// Returns an exited lifecycle option,
    /// 
    pub fn exited() -> Self {
        Self::Exit(None)
    }
}

impl Default for LifecycleOptions {
    fn default() -> Self {
        LifecycleOptions::Exit(Some(()))
    }
}
