mod runmd;
use std::ops::Deref;

use reality::{CustomAttribute, Parser};
pub use runmd::RunmdFile;
use specs::{Read, ReadStorage, SystemData};
use specs::prelude::*;

use crate::prelude::{Runtime, Host, Sequencer};

use super::{Workspace, Project, default_world};

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
    pub fn new_host(&self) -> Host {
        let WorkspaceSource(.., workspace, _, files) = self;

        let workspace = workspace.deref().clone().expect("should have a workspace");

        let world = Adhoc::compile_workspace(
            &workspace, 
            files.join(), 
            Some(self.new_parser())
        );

        let mut host = Host::from(world);
        host.link_sequences();

        host.prepare::<Adhoc>();
        host
    }
}

/// Ad-hoc project used internally by WorkspaceSource for compiling a host in an adhoc manner
/// 
#[derive(Default)]
struct Adhoc;

impl Project for Adhoc {
    fn interpret(_: &World, _: &reality::Block) {
        //
    }
}