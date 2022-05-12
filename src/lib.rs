use logos::Logos;
use parser::Lifecycle;
use std::collections::{BTreeMap, HashMap};
use std::fmt::{Debug, Display};

pub mod editor;
pub use self::editor::App;
pub use self::editor::EditorEvent;
pub use self::editor::EditorRuntime;

pub trait RuntimeState {
    type Error;
    type State: Default + RuntimeState + Clone + Sized;

    /// load should take an initial message, and override any
    /// existing state that exists
    fn load<S: AsRef<str> + ?Sized>(&self, init: &S) -> Self
    where
        Self: Sized;

    /// process is a function that should take a string message
    /// and return the next version of Self
    fn process<S: AsRef<str> + ?Sized>(&self, msg: &S) -> Result<Self::State, Self::Error>;

    /// process is a function that should take a string message
    /// and return the next version of Self
    fn process_with_args<S: AsRef<str> + ?Sized>(
        state: WithArgs<Self>,
        msg: &S,
    ) -> Result<Self::State, Self::Error>
    where
        Self: Clone + Default + RuntimeState<State = Self>,
    {
        Self::process(&state.get_state(), msg)
    }

    /// select decides which listener should be processed next
    fn select(&self, listener: Listener<Self::State>, current: &Event) -> bool {
        listener.event.get_phase_lifecycle() == current.get_phase_lifecycle()
            && listener.event.get_prefix_label() == current.get_prefix_label()
            && listener.event.get_payload() == current.get_payload()
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

impl Event {
    pub fn exit() -> Event {
        Event {
            lifecycle: "exit".to_string(),
            ..Default::default()
        }
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
            "{{ {}_{}; {}_{}; {}_{} }}",
            self.phase, self.lifecycle, self.prefix, self.label, self.payload.1, self.payload.0
        )
    }
}

impl Into<String> for Event {
    fn into(self) -> String {
        format!("{}", self)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Listener<T>
where
    T: Default + RuntimeState + Clone,
{
    event: Event,
    action: Action<T>,
    next: Option<Event>,
    extensions: Extensions,
}

#[derive(Debug, Clone, Default)]
pub struct Extensions {
    context: Option<Event>,
    tests: Vec<(String, String)>,
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

    fn get_args(&self) -> Vec<String> {
        self.args.to_vec()
    }

    /// called by the runtime to execute the tests
    /// if None is returned that means this extension skipped running anything
    fn run_tests<T: Default + Clone + RuntimeState>(&self, action: Action<T>) -> Option<bool> {
        if let Action::Thunk(thunk) = action {
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
    T: Default + Clone + RuntimeState,
{
    /// dispatch sends a message to state for processing
    /// if successful transitions to the event described in the transition expression
    pub fn dispatch<S: AsRef<str> + ?Sized>(
        &mut self,
        msg: &S,
        transition_expr: &S,
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
    pub fn update(&mut self, thunk: fn(&T, Option<Event>) -> (T, String)) -> &mut Extensions {
        self.action = Action::Thunk(thunk);

        &mut self.extensions
    }

    pub fn call<S: AsRef<str> + ?Sized>(&mut self, name: &S) -> &mut Extensions {
        self.action = Action::Call(name.as_ref().to_string());

        &mut self.extensions
    }
}

#[derive(Clone)]
pub enum ThunkFunc<T: Default + RuntimeState<State = T> + Clone> {
    Default(fn(&T, Option<Event>) -> (T, String)),
    WithArgs(fn(&WithArgs<T>, Option<Event>) -> (T, String)),
}

impl<T: Default> Default for ThunkFunc<T>
where
    T: Default + RuntimeState<State = T> + Clone,
{
    fn default() -> Self {
        ThunkFunc::Default(|_, _| (T::default(), "{ exit;; }".to_string()))
    }
}

impl<T> Debug for ThunkFunc<T>
where
    T: Default + RuntimeState<State = T> + Clone,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThunkFunc::Default(_) => f.debug_tuple("ThunkFunc::Default").finish(),
            ThunkFunc::WithArgs(_) => f.debug_tuple("ThunkFunc::WithArgs").finish(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Runtime<T>
where
    T: RuntimeState<State = T> + Default + Clone,
{
    listeners: Vec<Listener<T>>,
    calls: HashMap<String, ThunkFunc<T>>,
    state: Option<T>,
    current: Option<Event>,
}

#[derive(Debug, Clone, Default)]
pub struct WithArgs<T>
where
    T: RuntimeState<State = T> + Default + Clone,
{
    state: T,
    args: Vec<String>,
}

impl<T> WithArgs<T>
where
    T: RuntimeState<State = T> + Default + Clone,
{
    pub fn get_state(&self) -> T {
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
    args.iter().map(|a| Argument::lexer(a)).filter_map(|mut p| {
        if let Some(Argument::Variable(v)) = p.next() {
            Some(v)
        } else {
            None
        }
    }).for_each(|(var, value)|{
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

    let args: Vec<String> = args.iter().map(|a| Argument::lexer(a)).filter_map(|mut p| {
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

impl<T> RuntimeState for WithArgs<T>
where
    T: RuntimeState<State = T> + Default + Clone,
{
    type State = Self;

    type Error = <T as RuntimeState>::Error;

    fn load<S: AsRef<str> + ?Sized>(&self, init: &S) -> Self
    where
        Self: Sized,
    {
        Self {
            state: self.state.load(init),
            args: self.args.to_owned(),
        }
    }

    fn process<S: AsRef<str> + ?Sized>(&self, msg: &S) -> Result<Self::State, Self::Error> {
        let next = self.state.process(msg);

        match next {
            Ok(next) => Ok(Self {
                state: next,
                args: self.args.to_owned(),
            }),
            Err(e) => Err(e),
        }
    }
}

impl<T> Runtime<T>
where
    T: RuntimeState<State = T> + Default + Clone,
{
    /// context gets the current event context of this runtime
    pub fn context(&self) -> Event {
        self.current.clone().unwrap_or(Event::exit())
    }

    /// current gets the current state
    pub fn current(&self) -> &Option<T> {
        &self.state
    }

    /// init creates the default state to start the runtime with
    pub fn init() -> T {
        T::default()
    }

    pub fn reset(&mut self) {
        self.current = None;
        self.state = None;
    }

    /// on parses an event expression, and adds a new listener for that event
    /// this method returns an instance of the Listener for further configuration
    pub fn on<S: AsRef<str> + ?Sized>(&mut self, event_expr: &S) -> &mut Listener<T> {
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

    pub fn with_call<S: AsRef<str> + ?Sized>(
        &mut self,
        call_name: &S,
        thunk: fn(&T, Option<Event>) -> (T, String),
    ) -> Self {
        self.calls
            .insert(call_name.as_ref().to_string(), ThunkFunc::Default(thunk));

        self.clone()
    }

    pub fn with_call_args<S: AsRef<str> + ?Sized>(
        &mut self,
        call_name: &S,
        thunk: fn(&WithArgs<T>, Option<Event>) -> (T, String),
    ) -> Self {
        self.calls
            .insert(call_name.as_ref().to_string(), ThunkFunc::WithArgs(thunk));

        self.clone()
    }

    /// start begins the runtime starting with the initial event expression
    /// the runtime will continue to execute until it reaches the { exit;; } event
    pub fn start<S: AsRef<str> + ?Sized>(&mut self, init_expr: &S) {
        let mut processing = self.parse_event(init_expr);

        loop {
            processing = processing.process();

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
    pub fn parse_event<S: AsRef<str> + ?Sized>(&mut self, expr: &S) -> Self {
        let mut lexer = Lifecycle::lexer(expr.as_ref());

        if let Some(Lifecycle::Event(e)) = lexer.next() {
            self.current = Some(e)
        }

        self.clone()
    }

    /// gets the next listener that will be processed
    pub fn next_listener(&self) -> Option<Listener<T>> {
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
    pub fn process(&mut self) -> Self {
        let state = self.prepare_state();

        if let Some(l) = self.next_listener() {
            match &l.action {
                Action::Call(name) => match self.calls.get(name) {
                    Some(ThunkFunc::Default(thunk)) => {
                        let (next_s, next_e) = thunk(&state, self.current.clone());
                        let mut lex = Lifecycle::lexer(&next_e);

                        match lex.next() {
                            Some(Lifecycle::Event(event)) => {
                                self.current = Some(event);
                                self.state = Some(next_s);
                            }
                            Some(Lifecycle::Error) => {
                                eprintln!(
                                    "Error parsing event expression: {:?}, {}",
                                    lex.span(),
                                    next_e
                                );
                            }
                            _ => {}
                        }
                    }
                    Some(ThunkFunc::WithArgs(thunk)) => {
                        let with_args = WithArgs {
                            state: state.clone(),
                            args: l.extensions.get_args(),
                        };

                        let (next_s, next_e) = thunk(&with_args, self.current.clone());
                        let mut lex = Lifecycle::lexer(&next_e);

                        match lex.next() {
                            Some(Lifecycle::Event(event)) => {
                                self.current = Some(event);
                                self.state = Some(next_s);
                            }
                            Some(Lifecycle::Error) => {
                                eprintln!(
                                    "Error parsing event expression: {:?}, {}",
                                    lex.span(),
                                    next_e
                                );
                            }
                            _ => {}
                        }
                    }
                    _ => (),
                },
                Action::Thunk(thunk) => {
                    let (next_s, next_e) = thunk(&state, self.current.clone());
                    let mut lex = Lifecycle::lexer(&next_e);

                    match lex.next() {
                        Some(Lifecycle::Event(event)) => {
                            self.current = Some(event);
                            self.state = Some(next_s);
                        }
                        Some(Lifecycle::Error) => {
                            eprintln!(
                                "Error parsing event expression: {:?}, {}",
                                lex.span(),
                                next_e
                            );
                        }
                        _ => {}
                    }
                }
                Action::Dispatch(msg) => {
                    let process = if l.extensions.args.len() > 0 {
                        T::process_with_args(
                            WithArgs {
                                state,
                                args: l.extensions.get_args(),
                            },
                            &msg,
                        )
                    } else {
                        state.process(&msg)
                    };

                    if let Ok(n) = process {
                        self.state = Some(n);
                        if let Some(next) = &l.next {
                            self.current = Some(next.clone());
                        }
                    }
                }
                _ => {}
            }
        }

        Self {
            calls: self.calls.clone(),
            listeners: self.listeners.to_vec(),
            state: self.state.clone(),
            current: self.current.clone(),
        }
    }

    fn prepare_state(&self) -> T {
        let mut state = self.state.clone();

        match &state {
            None => state = Some(Self::init()),
            _ => {}
        };

        state.expect("state was just set")
    }

    /// (Extension) test runs tests defined in listener extensions,
    /// and panics if all tests do not pass. If all tests pass this function
    /// will return the Runtime for execution
    /// TODO: Currently only tests .update(), and .call() Listeners, need to implement for .dispatch() as well
    pub fn test(&self) -> Result<Self, RuntimeTestError> {
        let test_pass = self.listeners.iter().all(|l| {
            let extension = &l.extensions;

            let mut action = l.action.clone();

            if let Action::Call(name) = l.action.clone() {
                action = {
                    if let Some(ThunkFunc::Default(thunk)) = self.calls.get(&name) {
                        Action::Thunk(*thunk)
                    } else {
                        Action::Thunk(|s, _| (s.clone(), "{ exit;; }".to_string()))
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
            })
        } else {
            Err(RuntimeTestError {})
        }
    }
}

#[derive(Debug)]
pub struct RuntimeTestError;

#[derive(Clone)]
enum Action<T>
where
    T: Default,
{
    NoOp,
    Call(String),
    Dispatch(String),
    Thunk(fn(&T, Option<Event>) -> (T, String)),
}

impl<T> Default for Action<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::NoOp
    }
}

impl<T> Debug for Action<T>
where
    T: Default,
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
            r#"$TEST='test1234' --test abc -t test --te et32t --json { test: "test"; "test"} --test { test: "abc", test2: "value"} -$TEST='test1234'"#,
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
                r#"{ test: "test"; "test"}"#.to_string()
            ))),
            lex.next()
        );
        assert_eq!(
            Some(Argument::Flag((
                Flags::LongFlag("test".to_string()),
                r#"{ test: "abc", test2: "value"}"#.to_string()
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
