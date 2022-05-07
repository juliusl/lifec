use logos::{Lexer, Logos};
use std::fmt::{Debug, Display};

pub trait RuntimeState<'a> 
{
    type Error;
    type State: 'a + Default + RuntimeState<'a> + Clone + Sized;

    /// load should take an initial message, and override any
    /// existing state that exists
    fn load(&self, init: &'a str) -> Self where Self: Sized;

    /// process is a function that should take a string message
    /// and return the next version of Self
    fn process(&self, msg: &'a str) -> Result<Self::State, Self::Error>;

    /// select decides which listener should be processed next
    fn select(&self, listener: Listener<'a, Self::State>, current: &Event<'a>) -> bool 
    {
        listener.event.get_phase_lifecycle() == current.get_phase_lifecycle()
            && listener.event.get_prefix_label() == current.get_prefix_label()
            && listener.event.get_payload() == current.get_payload()
    }
}

#[derive(Default, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Event<'a> {
    phase: &'a str,
    lifecycle: &'a str,
    prefix: &'a str,
    label: &'a str,
    payload: Signal<'a>,
}

impl<'a> Event<'a> {
    pub fn exit() -> Event<'a> {
        Event {
            lifecycle: "exit",
            ..Default::default()
        }
    }

    pub fn dispatch(&self, msg: &'a str) -> Self {
        self.with_signal("yield").with_data(msg)
    }

    pub fn with_phase(&self, phase: &'a str) -> Self {
        self.with("phase", phase)
    }

    pub fn with_lifecycle(&self, lifecycle: &'a str) -> Self {
        self.with("lifecycle", lifecycle)
    }

    pub fn with_prefix(&self, prefix: &'a str) -> Self {
        self.with("prefix", prefix)
    }

    pub fn with_label(&self, label: &'a str) -> Self {
        self.with("label", label)
    }

    pub fn with_signal(&self, signal: &'a str) -> Self {
        self.with("signal", signal)
    }

    pub fn with_data(&self, data: &'a str) -> Self {
        self.with("data", data)
    }

    pub fn clear(&self) -> Self {
        Self {
            phase: "",
            lifecycle: "",
            prefix: "",
            label: "",
            payload: Signal("", ""),
        }
    }

    fn with(&self, property: &'static str, value: &'a str) -> Self {
        let Event {
            phase,
            lifecycle,
            prefix,
            label,
            payload: Signal(signal, data),
        } = self.clone();

        match property {
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

impl<'a> Display for Event<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ {}_{}; {}_{}; {}_{} }}", self.phase, self.lifecycle, self.prefix, self.label, self.payload.1, self.payload.0)
    }
}

#[derive(Default, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Signal<'a>(pub &'a str, pub &'a str);

fn from_event_data_property_prefix<'a>(
    lex: &mut Lexer<'a, EventData<'a>>,
) -> Option<(&'a str, &'a str)> {
    let slice = lex.slice();
    if let Some(sep) = slice.chars().position(|c| c == '_') {
        let prefix = &slice[..sep];
        let label = &slice[sep + 1..slice.len() - 1];
        Some((prefix, label))
    } else {
        None
    }
}

fn from_event_data_property<'a>(lex: &mut Lexer<'a, EventData<'a>>) -> Option<&'a str> {
    let slice = lex.slice();
    let slice = &slice[..slice.len() - 1];

    Some(slice)
}

fn from_event_data_signal<'a>(
    lex: &mut Lexer<'a, EventData<'a>>,
) -> Option<(&'a str, &'a str)> {
    let mut slice = lex.slice();
    let pos = slice.chars().position(|c| c == '_');
    let mut signal = "";

    if let Some(p) = pos {
        signal = &slice[..p];
        slice = &slice[p + 1..];
    }

    Some((signal, slice))
}

#[derive(Logos, Debug, Hash, Clone, PartialEq, PartialOrd)]
enum EventData<'a> {
    #[regex(r"(?:[a-zA-Z-]+);", from_event_data_property)]
    Property(&'a str),
    #[regex(r"(?:[a-zA-Z-]+)_(?:[a-zA-Z-0-9]+);", from_event_data_property_prefix)]
    PropertyWithPrefix((&'a str, &'a str)),
    #[regex(r"(?:[a-zA-Z-]*)_*(?:[+/=a-zA-Z0-9]+)", from_event_data_signal)]
    Signal((&'a str, &'a str)),
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
        EventData::lexer("{ init; player_1; abscawimim4fa430m} { yield_a344fa34f43 }");

    assert_eq!(Some(EventData::Property("init")), lexer.next());
    assert_eq!(
        Some(EventData::PropertyWithPrefix(("player", "1"))),
        lexer.next()
    );
    assert_eq!(
        Some(EventData::Signal(("", "abscawimim4fa430m"))),
        lexer.next()
    );

    match lexer.next() {
        Some(EventData::Signal(("yield", payload))) => {
            assert_eq!("a344fa34f43", payload);
        }
        _ => {
            assert!(false, "expected to parse a signal")
        }
    }
}

fn from_lifecycle_event<'a>(lex: &mut Lexer<'a, Lifecycle<'a>>) -> Option<Event<'a>> {
    let slice = lex.slice();
    let slice = &slice[1..slice.len() - 1];
    let mut event_data = EventData::lexer(slice);
    let mut lifecycle_phase = "action";
    let mut event_payload: &str;
    let mut event_signal = "";
    let lifecycle_event: &str;
    let event_prefix: &str;
    let event_label: &str;

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
        event_payload = &slice[event_data.span().end + 1..];
        event_prefix = prefix;
        event_label = label;
    } else {
        event_payload = &slice[event_data.span().end + 1..];
        event_prefix = "";
        event_label = "";
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
        payload: Signal(event_signal, event_payload.trim()),
    })
}

#[derive(Logos, Debug, PartialEq, PartialOrd, Clone, Hash)]
pub enum Lifecycle<'a> {
    #[regex(r"\{[+/=<>a-z;A-Z0-9 _-]+\}", from_lifecycle_event)]
    Event(Event<'a>),
    // Logos requires one token variant to handle errors,
    // it can be named anything you wish.
    #[error]
    // We can also use this variant to define whitespace,
    // or any other matches we wish to skip.
    #[regex(r"[ \t\n\f\r]+", logos::skip)]
    Error,
}

impl<'a> ToString for Lifecycle<'a> {
    fn to_string(&self) -> String {
        match self {
            Lifecycle::Event(Event {
                phase,
                lifecycle,
                prefix,
                label,
                payload: Signal("", data),
            }) => format!(
                "{{ {}_{}; {}_{}; {} }}",
                phase, lifecycle, prefix, label, data
            ),
            Lifecycle::Event(Event {
                phase,
                lifecycle,
                prefix,
                label,
                payload: Signal(signal, data),
            }) => format!(
                "{{ {}_{}; {}_{}; {}_{} }}",
                phase, lifecycle, prefix, label, signal, data
            ),
            _ => String::default(),
        }
    }
}

#[test]
fn test_lifecycle() {
    let mut lexer = Lifecycle::lexer("{ before_update; test_life; yield_13nmiafn3i } { after_update; test_life2; ok_13nmiafn3i } { after_update; test_life2; 13nmiafn3i }");
    assert_eq!(
        Some(Lifecycle::Event(Event {
            phase: "before",
            lifecycle: "update",
            prefix: "test",
            label: "life",
            payload: Signal("yield", "13nmiafn3i")
        })),
        lexer.next()
    );

    assert_eq!(
        Some(Lifecycle::Event(Event {
            phase: "after",
            lifecycle: "update",
            prefix: "test",
            label: "life2",
            payload: Signal("ok", "13nmiafn3i")
        })),
        lexer.next()
    );

    assert_eq!(
        Some(Lifecycle::Event(Event {
            phase: "after",
            lifecycle: "update",
            prefix: "test",
            label: "life2",
            payload: Signal("", "13nmiafn3i")
        })),
        lexer.next()
    );

    let test = Lifecycle::Event(Event {
        phase: "after",
        lifecycle: "update",
        prefix: "test",
        label: "life2",
        payload: Signal("test", "13nmiafn3i"),
    });

    assert_eq!(
        "{ after_update; test_life2; test_13nmiafn3i }",
        test.to_string()
    );

    let test = Lifecycle::Event(Event {
        phase: "after",
        lifecycle: "update",
        prefix: "test",
        label: "life2",
        payload: Signal("", "13nmiafn3i"),
    });

    assert_eq!("{ after_update; test_life2; 13nmiafn3i }", test.to_string());

    let mut lexer = Lifecycle::lexer("{ setup;; } { action_setup; _;   }");

    assert_eq!(
        Some(Lifecycle::Event(Event {
            phase: "action",
            lifecycle: "setup",
            prefix: "",
            label: "",
            payload: Signal("", "")
        })),
        lexer.next()
    );

    assert_eq!(
        Some(Lifecycle::Event(Event {
            phase: "action",
            lifecycle: "setup",
            prefix: "",
            label: "",
            payload: Signal("", "")
        })),
        lexer.next()
    );
    println!("{:?}", lexer.next());
}


#[derive(Clone)]
enum Action<'a, T>
where 
    T: Default
{
    NoOp,
    Dispatch(&'a str),
    Thunk(fn(&T, Option<Event<'a>>) -> (T, &'a str)),
}

impl<'a, T> Default for Action<'a, T> 
where 
    T: Default
{
    fn default() -> Self {
        Self::NoOp
    }
}

impl<'a, T> Debug for Action<'a, T> 
where
    T: Default
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoOp => write!(f, "NoOp"),
            Self::Dispatch(arg0) => f.debug_tuple("Dispatch").field(arg0).finish(),
            Self::Thunk(_) => write!(f, "thunk"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Listener<'a, T> 
where
    T: 'a + Default + RuntimeState<'a> + Clone
{
    event: Event<'a>,
    action: Action<'a, T>,
    next: Option<Event<'a>>,
    extensions: Extensions<'a, T>,
}

#[derive(Debug, Clone, Default)]
pub struct Extensions<'a, T>
where 
    T: Default
{
    action: Action<'a, T>,
    context: Option<Event<'a>>,
    tests: Vec<(&'a str, &'a str)>,
}

impl<'a, T> Extensions<'a, T> 
where 
    T: Default + Clone + RuntimeState<'a>
{
    /// sets the current event context which will be used for extension methods
    pub fn set_context(&mut self, context: Option<Event<'a>>) -> &mut Self {
        self.context = context;
        self
    }

    /// adds a test case to the extensions of the listener, 
    /// only relevant for testing Thunk's at the moment
    pub fn test(&mut self, init: &'a str, expected: &'a str) -> &mut Self {
        self.tests.push((init, expected));
        self
    }

    /// called by the runtime, this sets the action that was assigned
    /// to the listener this extension is assigned to
    fn init(&mut self, action: Action<'a, T>) -> &mut Self {
        self.action = action;
        self
    }

    /// called by the runtime to execute the tests 
    /// if None is returned that means this extension skipped running anything
    fn run_tests(&self) -> Option<bool> {
        if let Action::Thunk(thunk) = self.action.clone() {
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

impl<'a, T> Listener<'a, T> 
where
    T: Default + Clone + RuntimeState<'a>,
{
    /// dispatch sends a message to state for processing
    /// if successful transitions to the event described in the transition expression
    pub fn dispatch(&mut self, msg: &'static str, transition_expr: &'static str) -> &mut Extensions<'a, T> {
        self.action = Action::Dispatch(msg);

        let lexer = Lifecycle::lexer(transition_expr);
        let mut lexer = lexer;
        if let Some(Lifecycle::Event(e)) = lexer.next() {
            self.next = Some(e);
        }

        &mut self.extensions
    }

    /// update takes a function and creates a listener that will be passed the current state
    /// and event context for processing. The function must return the next state, and the next event expression
    /// to transition to
    pub fn update(&mut self, thunk: fn(&T, Option<Event<'a>>) -> (T, &'a str)) -> &mut Extensions<'a, T> {
        self.action = Action::Thunk(thunk);

        &mut self.extensions
    }
}

#[derive(Debug, Default)]
pub struct Runtime<'a, T>
where
    T: RuntimeState<'a, State = T> + Default + Clone,
{
    listeners: Vec<Listener<'a, T>>,
    state: Option<T>,
    current: Option<Event<'a>>,
}

impl<'a, T> Runtime<'a, T>
where
    T: RuntimeState<'a, State = T> + Default + Clone,
{
    /// context gets the current event context of this runtime
    pub fn context(&self) -> Event<'a> {
        self.current.clone().unwrap_or(Event::exit())
    }

    /// init creates the default state to start the runtime with
    pub fn init() -> T {
        T::default()
    }

    /// current gets the current state
    pub fn current(&self) -> &Option<T> {
        &self.state
    }

    /// on parses an event expression, and adds a new listener for that event
    /// this method returns an instance of the Listener for further configuration
    pub fn on(&mut self, event_expr: &'static str) -> &mut Listener<'a, T> {
        let mut lexer = Lifecycle::lexer(event_expr);

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
    pub fn start(self, init_expr: &'a str) {
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
    pub fn parse_event(mut self, expr: &'a str) -> Self {
        let mut lexer = Lifecycle::lexer(expr);

        if let Some(Lifecycle::Event(e)) = lexer.next() {
            self.current = Some(e)
        }

        self
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

        if let Some(l) = self.listeners
            .iter()
            .find(|l| state.select(l.to_owned().clone(), &self.context()))
        {
            match l.action {
                Action::Thunk(thunk) => {
                    let (next_s, next_e) = thunk(&state, self.current.clone());
                    let mut lex = Lifecycle::lexer(next_e);

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
                    if let Ok(n) = state.process(msg) {
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
    pub fn test(&'a mut self) -> &'a mut Self {
        let test_pass = self.listeners.iter().cloned().all(|l| {
            let mut extension = l.extensions;
            let extension = &mut extension;
            let extension = extension.init(l.action);
            if let Some(result) = extension.run_tests() {
                if !result {
                    eprintln!("error with listener on({})", l.event);
                } else {
                    eprintln!("{} passed.", l.event);
                }
                result
            } else {
                eprintln!("{} skipped.", l.event);
                true
            }
        });

        if test_pass {
            self 
        } else {
            panic!("did not pass all tests");
        }
    }
}
