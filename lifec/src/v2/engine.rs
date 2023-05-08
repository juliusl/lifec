use reality::v2::prelude::*;

/// Struct containing engine settings,
///
/// ```runmd
/// +   .symbol     Engine      # An engine is a sequence of at least two events, this root provides extensions for specifying event behavior
/// <>  .start                  # This extension indicates that the event should start immediately
/// :   after       .symbol     # Schedules this event to execute after the identifier specified in this property
/// :   spawn       .false      # This property indicates if event should be serialized, or if all should spawn at once
/// <>  .once                   # This extension indicates that the event should only start once
/// :   called      .false      # This property indicates if the event has been called
/// :   after       .symbol     # Schedules this event to execute after the identifier specified in this property
/// ```
///
#[reality::parse_docs]
#[derive(Clone, Debug)]
pub struct Engine {
    properties: Properties,
}

#[derive(Runmd, Debug, Component, Clone)]
#[storage(VecStorage)]
pub struct Event {
    after: String,
    name: String,
}

impl Event {
    fn new() -> Self {
        Self { after: String::new(), name: String::new() }
    }
}

impl Engine {
    /// Returns a new empty engine,
    ///
    pub fn new() -> Self {
        Engine {
            properties: Properties::empty(),
        }
    }

    /// Configures events to start only once,
    ///
    pub fn once(&self, name: &str, events: &Vec<String>) -> Property {
        let output = Properties::empty();

        for e in events.iter() {
            println!("event -- {name}.{e}");
            let key = format!("engine.once.{name}");
            if let Some(properties) = self
                .properties
                .property(key)
                .and_then(|p| p.as_properties())
            {
                let mut event = Event::new();
                event.visit_properties(&properties);

                println!("{:?}", event);
            }
        }

        Property::Properties(output.into())
    }

    /// Configures events to start,
    ///
    pub fn start(&self, name: &str, events: &Vec<String>) -> Property {
        let output = Properties::empty();

        for e in events.iter() {
            println!("event -- {name}.{e}");
            let key = format!("engine.start.{name}");

            if let Some(properties) = self
                .properties
                .property(key)
                .and_then(|p| p.as_properties())
            {
                let mut event = Event::new();
                event.visit_properties(&properties);
                println!("{:?}", event);
            } else {
                
            }
        }

        Property::Properties(output.into())
    }
}

impl Visitor for Engine {
    fn visit_property(&mut self, name: &str, property: &Property) {
        self.properties.visit_property(name, property);
    }

    fn visit_extension(&mut self, identifier: &Identifier) {
        println!("visiting --- {}", identifier);
    }
}

impl Visit for Engine {
    fn visit(&self, _: (), _: &mut impl Visitor) -> Result<()> {
        Ok(())
    }
}
