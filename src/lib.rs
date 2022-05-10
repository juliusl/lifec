use logos::Logos;
use parser::Lifecycle;
use std::fmt::{Debug, Display};

#[cfg(any(feature = "editor"))]
pub mod editor;
#[cfg(any(feature = "editor"))]
pub use self::editor::EditorEvent;
#[cfg(any(feature = "editor"))]
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
}

#[derive(Debug, Default, Clone)]
pub struct Runtime<T>
where
    T: RuntimeState<State = T> + Default + Clone,
{
    listeners: Vec<Listener<T>>,
    state: Option<T>,
    current: Option<Event>,
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

    /// process handles the internal logic
    /// based on the context, the state implementation selects the next listener
    pub fn process(&mut self) -> Self {
        let mut state = self.state.clone();

        match &state {
            None => state = Some(Self::init()),
            _ => {}
        };

        let state = state.unwrap();

        if let Some(l) = self
            .listeners
            .iter()
            .find(|l| state.select(l.to_owned().clone(), &self.context()))
        {
            match &l.action {
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
                    if let Ok(n) = state.process(&msg) {
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
            listeners: self.listeners.to_vec(),
            state: self.state.clone(),
            current: self.current.clone(),
        }
    }

    /// (Extension) test runs tests defined in listener extensions,
    /// and panics if all tests do not pass. If all tests pass this function
    /// will return the Runtime for execution
    pub fn test(&self) -> Result<Self, RuntimeTestError> {
        let test_pass = self.listeners.iter().all(|l| {
            let extension = &l.extensions;
            if let Some(result) = extension.run_tests(l.action.clone()) {
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
        #[regex(r"(?:[a-zA-Z-]+)_(?:[a-zA-Z-0-9]+);", from_event_data_property_prefix)]
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

    #[test]
    fn test_event_data() {
        let mut lexer =
            EventData::lexer("{ init; player_1; abscawimim4fa430m} { a344fa34f43_yield }");

        assert_eq!(Some(EventData::Property("init".to_string())), lexer.next());
        assert_eq!(
            Some(EventData::PropertyWithPrefix(("player".to_string(), "1".to_string()))),
            lexer.next()
        );
        assert_eq!(
            Some(EventData::Signal(("".to_string(), "abscawimim4fa430m".to_string()))),
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

        assert_eq!("{ after_update; test_life2; 13nmiafn3i_ }", test.to_string());

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
