use atlier::system::Attribute;
use logos::Logos;
use parser::Lifecycle;
use std::any::Any;
use std::collections::BTreeMap;
use std::fmt::{Debug, Display};

pub mod editor;
pub mod plugins;

mod state;
pub use state::AttributeGraph;

pub trait RuntimeDispatcher: AsRef<AttributeGraph> + AsMut<AttributeGraph> 
where
    Self: Sized
{
    type Error;

    /// dispatch_mut is a function that should take a string message that can mutate state
    /// and returns a result
    fn dispatch_mut(&mut self, msg: impl AsRef<str>) -> Result<(), Self::Error>;

    /// dispatch calls dispatch_mut on a clone of Self and returns the clone
    fn dispatch(&self, msg: impl AsRef<str>) -> Result<Self, Self::Error> 
    where
        Self: Clone
    {
        let mut next = self.to_owned();
        match next.dispatch_mut(msg) {
            Ok(_) => {
                Ok(next.to_owned())
            },
            Err(err) => Err(err),
        }
    }

    fn batch(&self, msg: impl AsRef<str>) -> Result<Self, Self::Error> 
    where 
        Self: Clone
    {
        let mut next = self.clone();
        for message in msg.as_ref().trim().split("\n").filter(|line| !line.is_empty()) {
             next = next.dispatch(message)?;
        }
    
        Ok(next)
    }

    fn batch_mut(&mut self, msg: impl AsRef<str>) -> Result<(), Self::Error> {
        for message in msg.as_ref().trim().split("\n").map(|line| line.trim()).filter(|line| !line.is_empty()) {
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

pub trait RuntimeState: Any + Sized + Clone + Sync + Default + Send + Display + From<AttributeGraph> {
    type Dispatcher: RuntimeDispatcher;

    /// setup runtime is called to configure calls and listeners for the runtime
    fn setup_runtime(&mut self, runtime: &mut Runtime<Self>) {
        runtime.with_call("save", |s, e| {
            if let Some(e) = e {
                (None, e.dispatch_load(s).to_string())
            } else {
                (None, "{ exit;; }".to_string())
            }
        });

        runtime.with_call("load", |s, e| {
            if let Some(e) = e {
                if let Some(payload) = e.read_payload() {
                    return (Some(s.load(payload)), "{ ok;; }".to_string());
                }
            }

            (None, "{ exit;; }".to_string())
        });

        runtime.with_call_mut("dispatch_mut", |s, e| {
            if let Some(msg) = e.and_then(|e| e.read_payload()) {
                match s.dispatcher_mut().dispatch_mut(&msg) {
                    Ok(_) => {
                        "{ ok;; }".to_string()
                    },
                    Err(_) => {
                        "{ error;; }".to_string()
                    }
                }
            } else {
                "{ exit;; }".to_string()
            }          
        });
    }

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
        
        next.state_mut()
            .merge(other.state());
        
        next
    }

    /// select decides which listener should be processed next
    fn select(&self, listener: Listener<Self>, current: &Event) -> bool {
        listener.event.get_phase_lifecycle() == current.get_phase_lifecycle() && listener.event.get_prefix_label() == current.get_prefix_label()
    }
}

#[derive(Default, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Event {
    phase: String,
    lifecycle: String,
    prefix: String,
    label: String,
    payload: Signal,
}

#[derive(Default, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Signal(pub String, pub String);

impl Signal {
    pub fn encode(&mut self, data: impl AsRef<str>, signal: impl AsRef<str>) {
        let encoded = base64::encode(data.as_ref().as_bytes());

        self.0 = encoded;
        self.1 = signal.as_ref().to_string();
    }

    pub fn decode(&self) -> Option<String> {
        if !self.0.is_empty() {
            base64::decode(&self.0).ok().and_then(|v| String::from_utf8(v).ok())
        } else {
            None
        }
    }
}

impl Event {
    pub fn exit() -> Event {
        Event {
            lifecycle: "exit".to_string(),
            ..Default::default()
        }
    }

    pub fn dispatch_err(&self, msg: impl AsRef<str>) -> Self {
        let mut next = self.clone();

        next.payload.encode(msg, "err");

        next
    }

    pub fn dispatch_msg(&self, msg: impl AsRef<str>) -> Self {
        let mut next = self.clone();

        next.payload.encode(msg, "msg");

        next
    }

    pub fn dispatch_load<S>(&self, state: &S) -> Self 
    where 
        S: RuntimeState
    {
        if let Some(saved) = state.save() {
            let mut next = self.clone();
    
            next.payload.encode(saved, "load");
            next
        } else {
            self.clone()
        }
    }

    pub fn payload(&self) -> &Signal {
        &self.payload
    }

    pub fn payload_mut(&mut self) -> &mut Signal {
        &mut self.payload
    }

    pub fn read_payload(&self) -> Option<String> {
        self.payload.decode()
    }

    pub fn dispatch<S: AsRef<str> + ?Sized>(&self, msg: &S) -> Self {
        self.with_signal("yield").with_data(msg)
    }

    pub fn with_phase<S: AsRef<str> + ?Sized>(&self, phase: &S) -> Self {
        self.with("phase", phase.as_ref())
    }

    pub fn with_lifecycle<S: AsRef<str> + ?Sized>(&self, lifecycle: &S) -> Self {
        self.with("lifecycle", lifecycle.as_ref())
    }

    pub fn with_prefix<S: AsRef<str> + ?Sized>(&self, prefix: &S) -> Self {
        self.with("prefix", prefix.as_ref())
    }

    pub fn with_label<S: AsRef<str> + ?Sized>(&self, label: &S) -> Self {
        self.with("label", label.as_ref())
    }

    pub fn with_signal<S: AsRef<str> + ?Sized>(&self, signal: &S) -> Self {
        self.with("signal", signal.as_ref())
    }

    pub fn with_data<S: AsRef<str> + ?Sized>(&self, data: &S) -> Self {
        self.with("data", data.as_ref())
    }

    pub fn clear(&self) -> Self {
        Self {
            ..Default::default()
        }
    }

    fn with<S: AsRef<str> + ?Sized>(&self, property: &S, value: &S) -> Self {
        let Event {
            phase,
            lifecycle,
            prefix,
            label,
            payload: Signal(signal, data),
        } = self.clone();

        let value = value.as_ref().to_string();

        match property.as_ref() {
            "phase" => Self {
                phase: value,
                lifecycle,
                prefix,
                label,
                payload: Signal(signal, data),
            },
            "lifecycle" => Event {
                phase,
                lifecycle: value,
                prefix,
                label,
                payload: Signal(signal, data),
            },
            "prefix" => Event {
                phase,
                lifecycle,
                prefix: value,
                label,
                payload: Signal(signal, data),
            },
            "label" => Event {
                phase,
                lifecycle,
                prefix,
                label: value,
                payload: Signal(signal, data),
            },
            "signal" => Event {
                phase,
                lifecycle,
                prefix,
                label,
                payload: Signal(value, data),
            },
            "data" => Event {
                phase,
                lifecycle,
                prefix,
                label,
                payload: Signal(signal, value),
            },
            _ => self.clone(),
        }
    }

    pub fn get_phase_lifecycle(&self) -> (&str, &str) {
        let Event {
            lifecycle, phase, ..
        } = self;

        (phase, lifecycle)
    }

    pub fn get_prefix_label(&self) -> (&str, &str) {
        let Event { prefix, label, .. } = self;

        (prefix, label)
    }

    pub fn get_payload(&self) -> (&str, &str) {
        let Event {
            payload: Signal(signal, data),
            ..
        } = self;

        (signal, data)
    }
}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ {}_{}; {}_{};",
            self.phase, self.lifecycle, self.prefix, self.label
        )?;

        if !self.payload.0.is_empty() && !self.payload.1.is_empty() {
            write!(f, " {}_{}", self.payload.1, self.payload.0,)
        } else {
            write!(f, " }}")
        }
    }
}

impl Into<String> for Event {
    fn into(self) -> String {
        format!("{}", self)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Listener<S>
where
    S: RuntimeState,
{
    pub event: Event,
    pub action: Action<S>,
    pub next: Option<Event>,
    pub extensions: Extensions,
}

#[derive(Debug, Clone, Default)]
pub struct Extensions {
    pub context: Option<Event>,
    pub args: Vec<String>,
    pub tests: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default)]
pub struct Runtime<S>
where
    S: RuntimeState,
{
    listeners: Vec<Listener<S>>,
    calls: BTreeMap<String, ThunkFunc<S>>,
    state: Option<S>,
    current: Option<Event>,
    attributes: AttributeGraph,
}

impl<S> From<S> for Runtime<S> 
where
    S: RuntimeState
{
    fn from(state: S) -> Self {
        Self {
            state: Some(state),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct WithArgs<S>
where
    S: RuntimeState,
{
    state: S,
    args: Vec<String>,
}

impl Extensions {
    /// sets the current event context which will be used for extension methods
    pub fn set_context(&mut self, context: Option<Event>) -> &mut Self {
        self.context = context;
        self
    }

    /// adds a test case to the extensions of the listener,
    /// only relevant for testing Thunk's at the moment
    pub fn test<S: AsRef<str> + ?Sized>(&mut self, init: &S, expected: &S) -> &mut Self {
        self.tests
            .push((init.as_ref().to_string(), expected.as_ref().to_string()));
        self
    }

    pub fn args<'a>(&mut self, args: &[&'a str]) -> &mut Self {
        self.args = args.iter().map(|a| a.to_string()).collect();

        self
    }

    pub fn get_args(&self) -> Vec<String> {
        self.args.to_vec()
    }

    /// called by the runtime to execute the tests
    /// if None is returned that means this extension skipped running anything
    fn run_tests<T: Default + Clone + RuntimeState>(&self, action: Action<T>) -> Option<bool> {
        if let Action::Thunk(ThunkFunc::Call(thunk)) = action {
            let mut pass = true;

            self.tests.iter().for_each(|(init, expected)| {
                // Load the desired state with init
                let state = T::default().load(init);

                // Call the thunk function, and get the next destination
                // check if this matches our expectation
                let (_, next) = thunk(&state, self.context.clone());
                pass = next == *expected;
                if !pass {
                    eprintln!("expected: {}, got: {}", *expected, next);
                }
            });

            Some(pass)
        } else {
            None
        }
    }
}

impl<T> Listener<T>
where
    T: RuntimeState,
{
    /// dispatch sends a message to state for processing
    /// if successful transitions to the event described in the transition expression
    pub fn dispatch(
        &mut self,
        msg: impl AsRef<str>,
        transition_expr: impl AsRef<str>,
    ) -> &mut Extensions {
        self.action = Action::Dispatch(msg.as_ref().to_string());

        let lexer = Lifecycle::lexer(transition_expr.as_ref());
        let mut lexer = lexer;
        if let Some(Lifecycle::Event(e)) = lexer.next() {
            self.next = Some(e);
        }

        &mut self.extensions
    }

    /// update takes a function and creates a listener that will be passed the current state
    /// and event context for processing. The function must return the next state, and the next event expression
    /// to transition to
    pub fn update(&mut self, thunk: fn(&T, Option<Event>) -> (Option<T>, String)) -> &mut Extensions {
        self.action = Action::Thunk(ThunkFunc::Call(thunk));

        &mut self.extensions
    }

    /// update_mut configures the listener to call thunk
    pub fn update_mut(&mut self, thunk: fn(&mut T, Option<Event>) -> String) -> &mut Extensions {
        self.action = Action::Thunk(ThunkFunc::CallMut(thunk));

        &mut self.extensions
    }

    /// call configures the listener to call a thunk function by name
    pub fn call(&mut self, name: impl AsRef<str>) -> &mut Extensions {
        self.action = Action::Call(name.as_ref().to_string());

        &mut self.extensions
    }
}

#[derive(Clone)]
pub enum ThunkFunc<S>
where
    S: RuntimeState,
{
    Call(fn(&S, Option<Event>) -> (Option<S>, String)),
    CallMut(fn(&mut S, Option<Event>) -> String),
}

impl<T: Default> Default for ThunkFunc<T>
where
    T: RuntimeState,
{
    fn default() -> Self {
        ThunkFunc::Call(|_, _| (None, "{ exit;; }".to_string()))
    }
}

impl<S> Debug for ThunkFunc<S>
where
    S: RuntimeState,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThunkFunc::Call(_) => f.debug_tuple("ThunkFunc::Default").finish(),
            ThunkFunc::CallMut(_) => f.debug_tuple("ThunkFunc::Mutable").finish(),
        }
    }
}

impl<S> WithArgs<S>
where
    S: RuntimeState,
{
    pub fn get_state(&self) -> S {
        self.state.clone()
    }

    pub fn get_args(&self) -> &Vec<String> {
        &self.args
    }

    pub fn parse_flags(&self) -> BTreeMap<String, String> {
        parse_flags(self.args.clone())
    }

    pub fn parse_variables(&self) -> BTreeMap<String, String> {
        parse_variables(self.args.clone())
    }
}

pub fn parse_variables(args: Vec<String>) -> BTreeMap<String, String> {
    use parser::Argument;

    let mut map = BTreeMap::<String, String>::default();
    args.iter()
        .map(|a| Argument::lexer(a))
        .filter_map(|mut p| {
            if let Some(Argument::Variable(v)) = p.next() {
                Some(v)
            } else {
                None
            }
        })
        .for_each(|(var, value)| {
            if let Some(old) = map.insert(var.clone(), value.clone()) {
                eprintln!(
                    "Warning: replacing flag {}, with {} -- original: {}",
                    var, value, old
                );
            }
        });

    map
}

pub fn parse_flags(args: Vec<String>) -> BTreeMap<String, String> {
    use parser::Argument;
    use parser::Flags;

    let args: Vec<String> = args
        .iter()
        .map(|a| Argument::lexer(a))
        .filter_map(|mut p| {
            if let Some(Argument::Variable(_)) = p.next() {
                None
            } else {
                Some(p.source().to_string())
            }
        })
        .collect();

    let arguments = args.join(" ");

    let mut arg_lexer = Argument::lexer(arguments.as_ref());
    let mut map = BTreeMap::<String, String>::default();

    loop {
        match arg_lexer.next() {
            Some(Argument::Flag((flag, value))) => match flag {
                Flags::ShortFlag(f) => {
                    if let Some(old) = map.insert(format!("-{}", f), value.clone()) {
                        eprintln!(
                            "Warning: replacing flag {}, with {} -- original: {}",
                            f, value, old
                        );
                    }
                }
                Flags::LongFlag(flag) => {
                    if let Some(old) = map.insert(format!("--{}", flag), value.clone()) {
                        eprintln!(
                            "Warning: replacing flag {}, with {} -- original: {}",
                            flag, value, old
                        );
                    }
                }
                _ => continue,
            },
            Some(Argument::Variable(_)) => continue,
            Some(Argument::Error) => continue,
            None => break,
        }
    }

    map
}

impl<T: RuntimeState> Display for WithArgs<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "with_args")
    }
}

impl<S> From<AttributeGraph> for WithArgs<S>
where
    S: RuntimeState
{
    fn from(_: AttributeGraph) -> Self {
        todo!();
    }
}

impl<S> RuntimeState for WithArgs<S>
where
    S: RuntimeState,
{
    type Dispatcher = AttributeGraph;

    fn load(&self, init: impl AsRef<str>) -> Self {
        Self {
            state: self.state.load(init),
            args: self.args.to_owned(),
        }
    }
}

impl<S> From<&mut S> for Runtime<S> 
where
    S: RuntimeState
{
    fn from(state: &mut S) -> Self {
        let mut default = Runtime::<S>::default();
        state.setup_runtime(&mut default);
        default
    }
}

impl<S> Runtime<S>
where
    S: RuntimeState,
{
    /// ensures that the runtime is returned after excuting a thunk with call_name, otherwise panic
    pub fn ensure_call(
        &self,
        call_name: impl AsRef<str>,
        state: Option<S>,
        args: Option<Vec<String>>,
    ) -> Self {
        if let None = self.calls.get(call_name.as_ref()) {
            panic!(
                "call: '{}' is not defined on this runtime",
                call_name.as_ref()
            );
        }

        self.after_call(call_name, state, args)
    }

    /// returns the runtime after calling a thunk with the specified name
    /// with optional initial state and initial args, otherwise with no args
    /// and current state of runtime
    pub fn after_call(
        &self,
        call_name: impl AsRef<str>,
        state: Option<S>,
        args: Option<Vec<String>>,
    ) -> Self {
        let mut clone = self.clone();

        let state = state.or(self.clone().state).unwrap_or(Self::init());

        clone.execute_call(
            call_name,
            state,
            args.and_then(|a| {
                Some(Extensions {
                    args: a,
                    context: Some(clone.context()),
                    tests: vec![],
                })
            }),
        );

        clone
    }

    /// context gets the current event context of this runtime
    pub fn context(&self) -> Event {
        self.current.clone().unwrap_or(Event::exit())
    }

    /// current gets the current state
    pub fn current(&self) -> &Option<S> {
        &self.state
    }

    /// init creates the default state to start the runtime with
    pub fn init() -> S {
        S::default()
    }

    pub fn reset(&mut self) {
        self.current = None;
        self.state = None;
    }

    pub fn reset_listeners(&mut self, keep_update_listeners: bool) {
        self.listeners = self
            .listeners
            .iter()
            .filter(|l| match l.action {
                Action::Thunk(_) => keep_update_listeners,
                _ => false,
            })
            .cloned()
            .collect();
    }

    pub fn get_listeners(&self) -> Vec<Listener<S>> {
        self.listeners.clone()
    }

    /// Adds an attribute to this runtime and returns itself for further mutation
    pub fn with_attribute(&mut self, attribute: Attribute) -> &mut Self {
        self.attribute(&attribute);
        self
    }

    /// This add's an attribute to the runtime
    pub fn attribute(&mut self, attribute: &Attribute) {
        self.attributes.copy_attribute(attribute);
    }

    /// on parses an event expression, and adds a new listener for that event
    /// this method returns an instance of the Listener for further configuration
    pub fn on(&mut self, event_expr: impl AsRef<str>) -> &mut Listener<S> {
        let mut lexer = Lifecycle::lexer(event_expr.as_ref());

        if let Some(Lifecycle::Event(e)) = lexer.next() {
            let listener = Listener {
                event: e,
                ..Default::default()
            };

            self.listeners.push(listener);
            self.listeners.last_mut().unwrap()
        } else {
            panic!()
        }
    }

    /// Inserts a thunk with call_name into this runtime.
    /// Returns the runtime for chaining.
    pub fn with_call(
        &mut self,
        call_name: impl AsRef<str>,
        thunk: fn(&S, Option<Event>) -> (Option<S>, String),
    ) -> &mut Self {
        self.calls
            .insert(call_name.as_ref().to_string(), ThunkFunc::Call(thunk));

        self
    }

    /// Inserts a mut thunk with call_name for this runtime.
    /// Returns the runtime for chaining.
    pub fn with_call_mut(
        &mut self,
        call_name: impl AsRef<str>,
        thunk: fn(&mut S, Option<Event>) -> String
    ) -> &mut Self {
        self.calls.insert(call_name.as_ref().to_string(), ThunkFunc::CallMut(thunk));
        self
    }

    /// start begins the runtime starting with the initial event expression
    /// the runtime will continue to execute until it reaches the { exit;; } event
    pub fn start(&mut self, init_expr: impl AsRef<str>) {
        let mut processing = self.parse_event(init_expr);

        loop {
            processing = processing.step();

            if !processing.can_continue() {
                break;
            }
        }
    }

    /// can_continue checks if the runtime can continue processing
    pub fn can_continue(&self) -> bool {
        let current = self.context();
        if let (_, "exit") = current.get_phase_lifecycle() {
            false
        } else {
            true
        }
    }

    /// parse_event parses the event and sets it as the current context
    pub fn parse_event(&mut self, expr: impl AsRef<str>) -> Self {
        let mut lexer = Lifecycle::lexer(expr.as_ref());

        if let Some(Lifecycle::Event(e)) = lexer.next() {
            self.current = Some(e)
        }

        self.clone()
    }

    /// gets the next listener that will be processed
    pub fn next_listener(&self) -> Option<Listener<S>> {
        let state = self.prepare_state();
        if let Some(listener) = self
            .listeners
            .iter()
            .find(|l| state.select(l.to_owned().clone(), &self.context()))
        {
            Some(listener.to_owned())
        } else {
            None
        }
    }

    /// process handles the internal logic
    /// based on the context, the state implementation selects the next listener
    pub fn step(&mut self) -> Self {
        let state = self.prepare_state();

        if let Some(l) = self.next_listener() {
            match &l.action {
                Action::Call(name) => {
                    self.execute_call(name, state, Some(l.extensions));
                }
                Action::Thunk(ThunkFunc::Call(thunk)) => {
                    let (next_s, next_e) = thunk(&state, self.current.clone());
                    if let Some(next_s) = next_s {
                        self.update(next_s, next_e);
                    }
                }
                Action::Dispatch(_msg) => {
                    // let process = if l.extensions.args.len() > 0 {
                    //     S::process_with_args(
                    //         WithArgs {
                    //             state,
                    //             args: l.extensions.get_args(),
                    //         },
                    //         &msg,
                    //     )
                    // } else {
                    //     state.dispatch(&msg)
                    // };

                    // if let Ok(n) = process {
                    //     self.state = Some(n);
                    //     if let Some(next) = &l.next {
                    //         self.current = Some(next.clone());
                    //     }
                    // }
                }
                _ => {}
            }
        }

        Self {
            calls: self.calls.clone(),
            listeners: self.listeners.to_vec(),
            state: self.state.clone(),
            current: self.current.clone(),
            attributes: self.attributes.clone(),
        }
    }

    /// (Extension) test runs tests defined in listener extensions,
    /// and panics if all tests do not pass. If all tests pass this function
    /// will return the Runtime for execution
    /// TODO: Currently only tests .update(), and .call() Listeners, need to implement for .dispatch() as well
    pub fn test(&self) -> Result<Self, RuntimeTestError> {
        let test_pass = self.listeners.iter().all(|l| {
            let extension = &l.extensions;

            let mut action = l.action.clone();

            if let Action::Call(name) = action {
                action = {
                    if let Some(thunk) = self.calls.get(&name) {
                        Action::Thunk(thunk.clone())
                    } else {
                        Action::Thunk(ThunkFunc::Call(|_, _| (None, "{ exit;; }".to_string())))
                    }
                }
            }

            if let Some(result) = extension.run_tests(action) {
                if !result {
                    eprintln!("error with listener on({})", l.event);
                    false
                } else {
                    eprintln!("{} passed.", l.event);
                    true
                }
            } else {
                eprintln!("{} skipped.", l.event);
                true
            }
        });

        if test_pass {
            Ok(Self {
                calls: self.calls.clone(),
                state: self.state.clone(),
                listeners: self.listeners.clone(),
                current: self.current.clone(),
                attributes: self.attributes.clone(),
            })
        } else {
            Err(RuntimeTestError {})
        }
    }

    fn prepare_state(&self) -> S {
        let mut state = self.state.clone();

        match &state {
            None => state = Some(Self::init()),
            _ => {}
        };

        state.expect("state was just set")
    }

    fn update(&mut self, next_state: S, next_event: String) {
        let mut lex = Lifecycle::lexer(&next_event);
        match lex.next() {
            Some(Lifecycle::Event(event)) => {
                self.current = Some(event);
                self.state = Some(next_state);
            }
            Some(Lifecycle::Error) => {
                eprintln!(
                    "Error parsing event expression: {:?}, {}",
                    lex.span(),
                    next_event
                );
            }
            _ => {}
        }
    }

    fn execute_call(
        &mut self,
        call_name: impl AsRef<str>,
        state: S,
        _extensions: Option<Extensions>,
    ) {
        match self.calls.get(call_name.as_ref()) {
            Some(ThunkFunc::Call(thunk)) => {
                let (next_s, next_e) = thunk(&state, self.current.clone());
                if let Some(next_s) = next_s {
                    self.update(next_s, next_e);
                }
            }
            _ => (),
        };
    }
}

#[derive(Debug)]
pub struct RuntimeTestError;

#[derive(Clone)]
pub enum Action<S>
where
    S: RuntimeState,
{
    NoOp,
    Call(String),
    Dispatch(String),
    Thunk(ThunkFunc<S>),
}

impl<S> Default for Action<S>
where
    S: RuntimeState,
{
    fn default() -> Self {
        Self::NoOp
    }
}

impl<S> Debug for Action<S>
where
    S: RuntimeState,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoOp => write!(f, "NoOp"),
            Self::Dispatch(arg0) => f.debug_tuple("Dispatch").field(arg0).finish(),
            Self::Thunk(_) => write!(f, "thunk"),
            Self::Call(arg0) => f.debug_tuple("Call").field(arg0).finish(),
        }
    }
}

mod parser {
    use crate::{Event, Signal};
    use logos::{Lexer, Logos};
    use std::fmt::Debug;

    #[derive(Logos, Debug, Hash, Clone, PartialEq, PartialOrd)]
    pub enum EventData {
        #[regex(r"(?:[a-zA-Z-]+);", from_event_data_property)]
        Property(String),
        #[regex(r"(?:[a-zA-Z-]+)*_(?:[a-zA-Z-0-9]+);", from_event_data_property_prefix)]
        PropertyWithPrefix((String, String)),
        #[regex(r"(?:[+/=a-zA-Z0-9]+_*(?:[a-zA-Z-]*))", from_event_data_signal)]
        Signal((String, String)),
        // Logos requires one token variant to handle errors,
        // it can be named anything you wish.
        #[error]
        // We can also use this variant to define whitespace,
        // or any other matches we wish to skip.
        #[regex(r"[ \t\n\f{}]+", logos::skip)]
        Error,
    }

    fn from_long_flag(lex: &mut Lexer<Flags>) -> Option<String> {
        Some(lex.slice()[2..].to_string())
    }

    fn from_short_flag(lex: &mut Lexer<Flags>) -> Option<String> {
        Some(lex.slice()[1..].to_string())
    }

    fn from_argument(lex: &mut Lexer<Argument>) -> Option<(Flags, String)> {
        let mut flag_l = Flags::lexer(lex.slice());
        if let Some(flag) = flag_l.next() {
            let value = &lex.slice()[flag_l.slice().len() + 1..];

            Some((flag, value.trim().to_string()))
        } else {
            None
        }
    }

    fn from_variable(lex: &mut Lexer<Argument>) -> Option<(String, String)> {
        let eq_pos = lex
            .slice()
            .chars()
            .position(|c| c == '=')
            .expect("lexer shouldn't have tried to parse this if there wasn't an =");

        let mut variable = &lex.slice()[..eq_pos];
        if &lex
            .slice()
            .chars()
            .nth(0)
            .expect("there should be a character at 0")
            == &'-'
        {
            variable = &variable[1..];
        }

        let value = &lex.slice()[eq_pos + 1..];

        Some((variable.trim().to_string(), value.trim().to_string()))
    }

    #[derive(Logos, Debug, Hash, Clone, PartialEq, PartialOrd)]
    pub enum Flags {
        #[regex(r#"--[a-zA-Z0-9-]+"#, from_long_flag)]
        LongFlag(String),
        #[regex(r"-[a-zA-Z0-9]", from_short_flag)]
        ShortFlag(String),
        // Logos requires one token variant to handle errors,
        // it can be named anything you wish.
        #[error]
        // We can also use this variant to define whitespace,
        // or any other matches we wish to skip.
        #[regex(r"[ \t\n\f{}]+", logos::skip)]
        Error,
    }

    #[derive(Logos, Debug, Hash, Clone, PartialEq, PartialOrd)]
    pub enum Argument {
        #[regex(
            r#"[-]*[-]+[-a-zA-Z0-9]+ (:?[{][-\w\d ;"':,|+(*&^%$#@!`)=\[\]\\/><']*[}]|['][-\w\d ;":,|+(*&^%{}$#@!`)=\[\]\\/><]*[']|["][-\w\d ;:,|+(*&^%{}$#@!`)=\[\]\\/><']*["]|[^-][-\w\d;"':,|+(*&^%$#@!`)=\[\]\\/><']*)?"#,
            from_argument
        )]
        Flag((Flags, String)),
        #[regex(
            r#"[-]?[$][A-Z_]+=(:?[-\w\d{} "':,|+(*&^%#@!`)=\[\]\\/><]*|['][-\w\d{} ":,|+(*&^%#@!`)=\[\]\\/><]*[']|["][-\w\d{} ':,|+(*&^%#@!`)=\[\]\\/><-]*["];)?"#,
            from_variable
        )]
        Variable((String, String)),
        // Logos requires one token variant to handle errors,
        // it can be named anything you wish.
        #[error]
        // We can also use this variant to define whitespace,
        // or any other matches we wish to skip.
        #[regex(r"[ \t\n\f]+", logos::skip)]
        Error,
    }

    #[test]
    fn test_arguments() {
        let mut lex = Argument::lexer(
            r#"$TEST='test1234' --test abc -t test --te et32t --json '{ test: "test"; "test"}' --test '{ test: "abc", test2: "value"}' -$TEST='test1234'"#,
        );
        assert_eq!(
            Some(Argument::Variable((
                "$TEST".to_string(),
                r#"'test1234'"#.to_string()
            ))),
            lex.next()
        );
        assert_eq!(
            Some(Argument::Flag((
                Flags::LongFlag("test".to_string()),
                "abc".to_string()
            ))),
            lex.next()
        );
        assert_eq!(
            Some(Argument::Flag((
                Flags::ShortFlag("t".to_string()),
                "test".to_string()
            ))),
            lex.next()
        );
        assert_eq!(
            Some(Argument::Flag((
                Flags::LongFlag("te".to_string()),
                "et32t".to_string()
            ))),
            lex.next()
        );
        assert_eq!(
            Some(Argument::Flag((
                Flags::LongFlag("json".to_string()),
                r#"'{ test: "test"; "test"}'"#.to_string()
            ))),
            lex.next()
        );
        assert_eq!(
            Some(Argument::Flag((
                Flags::LongFlag("test".to_string()),
                r#"'{ test: "abc", test2: "value"}'"#.to_string()
            ))),
            lex.next()
        );
        assert_eq!(
            Some(Argument::Variable((
                "$TEST".to_string(),
                r#"'test1234'"#.to_string()
            ))),
            lex.next()
        );
    }

    #[test]
    fn test_event_data() {
        let mut lexer =
            EventData::lexer("{ init; player_1; abscawimim4fa430m} { a344fa34f43_yield }");

        assert_eq!(Some(EventData::Property("init".to_string())), lexer.next());
        assert_eq!(
            Some(EventData::PropertyWithPrefix((
                "player".to_string(),
                "1".to_string()
            ))),
            lexer.next()
        );
        assert_eq!(
            Some(EventData::Signal((
                "".to_string(),
                "abscawimim4fa430m".to_string()
            ))),
            lexer.next()
        );

        match lexer.next() {
            Some(EventData::Signal((signal, payload))) => {
                assert_eq!("yield", signal);
                assert_eq!("a344fa34f43", payload);
            }
            _ => {
                assert!(false, "expected to parse a signal")
            }
        }
    }

    #[derive(Logos, Debug, PartialEq, PartialOrd, Clone, Hash)]
    pub enum Lifecycle {
        #[regex(r"\{[+/=<>a-z;A-Z0-9 _-]+\}", from_lifecycle_event)]
        Event(Event),
        // Logos requires one token variant to handle errors,
        // it can be named anything you wish.
        #[error]
        // We can also use this variant to define whitespace,
        // or any other matches we wish to skip.
        #[regex(r"[ \t\n\f\r]+", logos::skip)]
        Error,
    }

    impl ToString for Lifecycle {
        fn to_string(&self) -> String {
            match self {
                Lifecycle::Event(Event {
                    phase,
                    lifecycle,
                    prefix,
                    label,
                    payload: Signal(signal, data),
                }) => format!(
                    "{{ {}_{}; {}_{}; {}_{} }}",
                    phase, lifecycle, prefix, label, data, signal
                ),
                _ => String::default(),
            }
        }
    }

    #[test]
    fn test_lifecycle() {
        let mut lexer = Lifecycle::lexer("{ before_update; test_life; 13nmiafn3i_yield } { after_update; test_life2; 13nmiafn3i_ok } { after_update; test_life2; 13nmiafn3i }");
        assert_eq!(
            Some(Lifecycle::Event(Event {
                phase: "before".to_string(),
                lifecycle: "update".to_string(),
                prefix: "test".to_string(),
                label: "life".to_string(),
                payload: Signal("yield".to_string(), "13nmiafn3i".to_string())
            })),
            lexer.next()
        );

        assert_eq!(
            Some(Lifecycle::Event(Event {
                phase: "after".to_string(),
                lifecycle: "update".to_string(),
                prefix: "test".to_string(),
                label: "life2".to_string(),
                payload: Signal("ok".to_string(), "13nmiafn3i".to_string())
            })),
            lexer.next()
        );

        assert_eq!(
            Some(Lifecycle::Event(Event {
                phase: "after".to_string(),
                lifecycle: "update".to_string(),
                prefix: "test".to_string(),
                label: "life2".to_string(),
                payload: Signal("".to_string(), "13nmiafn3i".to_string())
            })),
            lexer.next()
        );

        let test = Lifecycle::Event(Event {
            phase: "after".to_string(),
            lifecycle: "update".to_string(),
            prefix: "test".to_string(),
            label: "life2".to_string(),
            payload: Signal("test".to_string(), "13nmiafn3i".to_string()),
        });

        assert_eq!(
            "{ after_update; test_life2; 13nmiafn3i_test }",
            test.to_string()
        );

        let test = Lifecycle::Event(Event {
            phase: "after".to_string(),
            lifecycle: "update".to_string(),
            prefix: "test".to_string(),
            label: "life2".to_string(),
            payload: Signal("".to_string(), "13nmiafn3i".to_string()),
        });

        assert_eq!(
            "{ after_update; test_life2; 13nmiafn3i_ }",
            test.to_string()
        );

        let mut lexer = Lifecycle::lexer("{ setup;; } { action_setup; _;   }");

        assert_eq!(
            Some(Lifecycle::Event(Event {
                phase: "action".to_string(),
                lifecycle: "setup".to_string(),
                prefix: "".to_string(),
                label: "".to_string(),
                payload: Signal("".to_string(), "".to_string())
            })),
            lexer.next()
        );

        assert_eq!(
            Some(Lifecycle::Event(Event {
                phase: "action".to_string(),
                lifecycle: "setup".to_string(),
                prefix: "".to_string(),
                label: "".to_string(),
                payload: Signal("".to_string(), "".to_string())
            })),
            lexer.next()
        );
        println!("{:?}", lexer.next());
    }

    fn from_event_data_property_prefix(lex: &mut Lexer<EventData>) -> Option<(String, String)> {
        let slice = lex.slice();
        if let Some(sep) = slice.chars().position(|c| c == '_') {
            let prefix = &slice[..sep];
            let label = &slice[sep + 1..slice.len() - 1];
            Some((prefix.to_string(), label.to_string()))
        } else {
            None
        }
    }

    fn from_event_data_property(lex: &mut Lexer<EventData>) -> Option<String> {
        let slice = lex.slice();
        let slice = &slice[..slice.len() - 1];

        Some(slice.to_string())
    }

    fn from_event_data_signal(lex: &mut Lexer<EventData>) -> Option<(String, String)> {
        let mut slice = lex.slice();
        let pos = slice.chars().position(|c| c == '_');
        let mut signal = "";

        if let Some(p) = pos {
            signal = &slice[p + 1..];
            slice = &slice[..p];
        }

        Some((signal.to_string(), slice.to_string()))
    }

    fn from_lifecycle_event(lex: &mut Lexer<Lifecycle>) -> Option<Event> {
        let slice = lex.slice();
        let slice = &slice[1..slice.len() - 1];
        let mut event_data = EventData::lexer(slice);
        let mut lifecycle_phase = "action".to_string();
        let mut event_payload: String;
        let mut event_signal = "".to_string();
        let lifecycle_event: String;
        let event_prefix: String;
        let event_label: String;

        match event_data.next() {
            Some(EventData::Property(event)) => {
                lifecycle_event = event;
            }
            Some(EventData::PropertyWithPrefix((phase, event))) => {
                lifecycle_phase = phase;
                lifecycle_event = event;
            }
            _ => return None,
        };

        if let Some(EventData::PropertyWithPrefix((prefix, label))) = event_data.next() {
            event_payload = slice[event_data.span().end + 1..].to_string();
            event_prefix = prefix;
            event_label = label;
        } else {
            event_payload = slice[event_data.span().end + 1..].to_string();
            event_prefix = "".to_string();
            event_label = "".to_string();
        }

        match event_data.next() {
            Some(EventData::Signal((signal, payload))) => {
                event_signal = signal;
                event_payload = payload;
            }
            _ => {}
        }

        Some(Event {
            phase: lifecycle_phase,
            lifecycle: lifecycle_event,
            prefix: event_prefix,
            label: event_label,
            payload: Signal(event_signal, event_payload.trim().to_string()),
        })
    }
}
