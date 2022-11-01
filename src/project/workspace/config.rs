use std::collections::HashMap;
use std::ops::Deref;

use atlier::system::Value;
use reality::SpecialAttribute;
use specs::prelude::*;
use specs::SystemData;

use crate::prelude::*;

/// Special attribute to customize properties from the root block of a workspace,
///
#[derive(SystemData)]
pub struct Config<'a> {
    entity_map: Read<'a, HashMap<String, Entity>>,
    workspace: Read<'a, Option<Workspace>>,
    entities: Entities<'a>,
    blocks: ReadStorage<'a, Block>,
    events: ReadStorage<'a, Event>,
    graphs: WriteStorage<'a, AttributeGraph>,
}

impl<'a> Config<'a> {
    /// Scans the root block for configs,
    ///
    pub fn scan_root(&self) -> Vec<BlockIndex> {
        let Config { entities, blocks, .. } = self;
        let root_block = entities.entity(0);
        let root_block = blocks.get(root_block).expect("should have root block");
        let mut configs = vec![];
        for config in root_block
            .index()
            .iter()
            .filter(|r| r.root().name().ends_with("config"))
        {
            configs.push(config.clone());
        }
        configs
    }

    /// Finds the entity that needs to be configured and applies the config,
    /// 
    pub fn find_apply(&mut self, config: &BlockIndex) {
        let Config { entity_map, .. } = self;

        let entity_map = entity_map.clone();

        if let Value::Symbol(expression) = config.root().value() {
            if let Some((name, symbol)) = expression.split_once('.') {
                let expression = format!("{name} {symbol}");

                if let Some(event) = entity_map.get(&expression) {
                    self.apply_config(*event, config.properties())
                }
            }
        }
    }

    /// Applies a config to the graphs related to event,
    ///
    pub fn apply_config(&mut self, event: Entity, config: &BlockProperties) {
        let Config { events, graphs, .. } = self;

        if let Some(sequence) = events.get(event).and_then(|e| e.sequence()) {
            for p in sequence.iter_entities() {
                if let Some(graph) = graphs.get_mut(p) {
                    graph.apply(config.clone());
                }
            }
        }
    }

    /// Apply config to state,
    /// 
    pub fn apply(&mut self) {
        let tag = self.workspace.deref().as_ref().and_then(|w| w.tag()).cloned();
        let configs = self.scan_root();

        if let Some(config) = configs.iter().find(|c| c.root().name() == "config") {
            self.find_apply(config);
        }

        if let Some(tag) = tag {
            for config in configs
                .clone()
                .iter()
                .filter(|c| c.root().name().starts_with(&tag))
            {
                self.find_apply(config);
            }
        }
    }
}

impl<'a> SpecialAttribute for Config<'a> {
    fn ident() -> &'static str {
        "config"
    }

    /// Parses a set of properties to insert into state,
    ///
    /// Content is a uri expression that resolves to the graph that will be configured,
    ///
    fn parse(parser: &mut reality::AttributeParser, content: impl AsRef<str>) {
        if let Some(name) = parser.name() {
            if name != "config" {
                parser.set_name(format!("{name}.config"));
                parser.set_value(Value::Symbol(content.as_ref().to_string()));
            }
        }
    }
}
