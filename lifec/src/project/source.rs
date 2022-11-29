mod runmd;
use std::ops::Deref;

use reality::{CustomAttribute, Parser};
pub use runmd::RunmdFile;
use specs::prelude::*;
use specs::{Read, ReadStorage, SystemData};

use crate::prelude::{Host, Runtime, Sequencer};

use super::{default_world, Listener, Project, Workspace};

/// System data for workspace source and resources used to parse the world,
///
#[derive(SystemData)]
pub struct WorkspaceSource<'a>(
    Read<'a, Runtime>,
    Read<'a, Vec<CustomAttribute>>,
    Read<'a, Option<Workspace>>,
    Entities<'a>,
    ReadStorage<'a, RunmdFile>,
);

impl<'a> WorkspaceSource<'a> {
    /// Returns a new runmd parser w/ the world's current runtime and custom attributes,
    ///
    pub fn new_parser(&self) -> Parser {
        let WorkspaceSource(runtime, custom_attributes, ..) = self;
        let mut world = default_world();
        world.insert(runtime.deref().clone());

        let mut parser = Parser::new_with(world);
        for c in custom_attributes.iter() {
            parser.add_custom_attribute(c.clone());
        }
        parser
    }

    /// Compiles a new host from workspace source,
    ///
    pub fn new_host<P>(&self) -> Host
    where
        P: Project,
    {
        let WorkspaceSource(.., workspace, _, files) = self;

        let workspace = workspace.deref().clone().expect("should have a workspace");

        let world = P::compile_workspace(&workspace, files.join(), Some(self.new_parser()));

        let mut host = Host::from(world);
        host.link_sequences();
        host.build_appendix();
        host
    }

    /// Returns a host with a listener enabled,
    ///
    pub fn new_host_with_listener<P, L>(&self) -> Host
    where
        P: Project,
        L: Listener,
    {
        let WorkspaceSource(.., workspace, _, files) = self;

        let workspace = workspace.deref().clone().expect("should have a workspace");

        let world = P::compile_workspace(&workspace, files.join(), Some(self.new_parser()));

        let mut host = Host::from(world);
        host.link_sequences();
        host.build_appendix();
        host.enable_listener::<L>();
        host
    }
}
