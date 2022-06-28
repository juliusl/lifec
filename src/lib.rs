use atlier::system::App;
use imgui::{ChildWindow, Window, Ui};
use plugins::{Engine, Plugin, Project, ThunkContext, Event};
use specs::{Component, System, World, Entity};
use std::fmt::Display;
use std::{any::Any, collections::BTreeMap};

pub mod editor;
pub mod plugins;

mod state;
pub use state::AttributeGraph;

pub trait RuntimeDispatcher: AsRef<AttributeGraph> + AsMut<AttributeGraph>
where
    Self: Sized,
{
    type Error;

    /// dispatch_mut is a function that should take a string message that can mutate state
    /// and returns a result
    fn dispatch_mut(&mut self, msg: impl AsRef<str>) -> Result<(), Self::Error>;

    /// dispatch calls dispatch_mut on a clone of Self and returns the clone
    fn dispatch(&self, msg: impl AsRef<str>) -> Result<Self, Self::Error>
    where
        Self: Clone,
    {
        let mut next = self.to_owned();
        match next.dispatch_mut(msg) {
            Ok(_) => Ok(next.to_owned()),
            Err(err) => Err(err),
        }
    }

    fn batch(&self, msgs: impl AsRef<str>) -> Result<Self, Self::Error>
    where
        Self: Clone,
    {
        let mut next = self.clone();
        for message in msgs
            .as_ref()
            .trim()
            .lines()
            .filter(|line| !line.trim().is_empty())
        {
            next = next.dispatch(message)?;
        }

        Ok(next)
    }

    fn batch_mut(&mut self, msg: impl AsRef<str>) -> Result<(), Self::Error> {
        for message in msg
            .as_ref()
            .trim()
            .split("\n")
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
        {
            self.dispatch_mut(message)?;
        }
        Ok(())
    }

    /// Dispatch a batch of messages from a file.
    fn from_file(&mut self, path: impl AsRef<str>) -> Result<(), Self::Error> {
        use std::fs;

        if let Some(initial_setup) = fs::read_to_string(path.as_ref()).ok() {
            self.batch_mut(initial_setup)?;
        }

        Ok(())
    }
}

pub trait RuntimeState:
    Any + Sized + Clone + Sync + Default + Send + Display + From<AttributeGraph>
{
    type Dispatcher: RuntimeDispatcher;

    // /// try to save the current state to a String
    fn save(&self) -> Option<String> {
        match serde_json::to_string(self.state()) {
            Ok(val) => Some(val),
            Err(_) => None,
        }
    }

    /// load should take the serialized form of this state
    /// and create a new instance of Self
    fn load(&self, init: impl AsRef<str>) -> Self {
        if let Some(attribute_graph) = serde_json::from_str::<AttributeGraph>(init.as_ref()).ok() {
            Self::from(attribute_graph)
        } else {
            self.clone()
        }
    }

    /// Returns a mutable dispatcher for this runtime state
    fn dispatcher_mut(&mut self) -> &mut Self::Dispatcher {
        todo!("dispatcher is not implemented for runtime state")
    }

    // Returns the dispatcher for this runtime state
    fn dispatcher(&self) -> &Self::Dispatcher {
        todo!("dispatcher is not implemented for runtime state")
    }

    // Returns the current state from the dispatcher
    fn state(&self) -> &AttributeGraph {
        self.dispatcher().as_ref()
    }

    // Returns the current state as mutable from dispatcher
    fn state_mut(&mut self) -> &mut AttributeGraph {
        self.dispatcher_mut().as_mut()
    }

    /// merge_with merges a clone of self with other
    fn merge_with(&self, other: &Self) -> Self {
        let mut next = self.clone();

        next.state_mut().merge(other.state());

        next
    }
}

#[derive(Clone)]
pub struct Runtime {
    project: Project,
    create: BTreeMap<String, fn(&World, fn(&mut ThunkContext)) -> Entity>,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            project: Default::default(),
            create: BTreeMap::default(),
        }
    }
}

impl Runtime {
    /// returns a runtime from a project
    pub fn new(project: Project) -> Self {
        Self {
            project,
            create: BTreeMap::default(),
        }
    }

    /// install an engine into the runtime
    pub fn install<E, P>(&mut self)
    where
        E: Engine,
        P: Plugin<ThunkContext> + Component + Send + Default,
    {
        let event = E::event::<P>();
        self.create.insert(event.to_string(), E::create::<P>);

        println!("installed {}", event.to_string());
    }

    /// initialize and configure an instance of an installed engine, corresponding to an event
    /// this is a no-op if the corresponding engine is not installed
    pub fn create(&self, world: &World, event: &Event, config_fn: fn(&mut ThunkContext)) -> Option<Entity> {
        let key = event.to_string();

        if let Some(create_fn) = self.create.get(&key) {
            Some((create_fn)(world, config_fn))
        } else {
            None
        }
    }

    /// returns a runtime state generated from the current project
    pub fn state<S>(&self) -> S
    where
        S: RuntimeState,
    {
        S::from(self.project.as_ref().clone())
    }
}

impl<'a> System<'a> for Runtime {
    type SystemData = (
    );

    fn run(&mut self, _: Self::SystemData) {
    }
}

impl Runtime {
    fn menu(&mut self, ui: &Ui) {
        ui.menu("Edit", ||{
            for (event_name, _) in self.create.iter() {
                ui.menu(event_name, || {

                })
            }
        });
    }
}

impl App for Runtime {
    fn name() -> &'static str {
        "runtime"
    }

    fn window_size() -> &'static [f32; 2] {
        &[1500.0, 720.0]
    }

    fn edit_ui(&mut self, ui: &imgui::Ui) {
        Window::new(format!(
            "Runtime - hash: {}",
            self.project.as_ref().hash_code()
        ))
        .size(*Self::window_size(), imgui::Condition::Appearing)
        .menu_bar(true)
        .build(ui, || {
            ui.menu_bar(|| {
                let project = &mut self.project;
                project.edit_project_menu(ui);

                self.menu(ui);
            });

            let project = &mut self.project;
            if let Some(tabbar) = ui.tab_bar("runtime_tabs") {
                for (_, block) in project.iter_block_mut().enumerate() {
                    let (block_name, block) = block;

                    let thunk_symbol = if let Some(thunk) = block.get_block("thunk") {
                        thunk.find_text("thunk_symbol")
                    } else {
                        None
                    };

                    let thunk_symbol = thunk_symbol.unwrap_or("entity".to_string());

                    if let Some(token) = ui.tab_item(format!("{} {}", thunk_symbol, block_name)) {
                        ui.group(|| {
                            block.edit_block_view(true, ui);
                            ChildWindow::new(&format!("table_view_{}", block_name))
                                .size([0.0, 0.0])
                                .build(ui, || {
                                    block.edit_block_table_view(ui);
                                });
                        });

                        token.end();
                    }
                }
                tabbar.end();
            }
        });
    }

    fn display_ui(&self, _: &imgui::Ui) {}
}
