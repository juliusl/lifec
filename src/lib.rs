use logos::{Lexer, Logos};
use std::fmt::Debug;

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
enum Action<'a, T> {
    NoOp,
    Dispatch(&'a str),
    Thunk(fn(&T, Option<Event<'a>>) -> (T, &'a str)),
}

impl<'a, T> Debug for Action<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoOp => write!(f, "NoOp"),
            Self::Dispatch(arg0) => f.debug_tuple("Dispatch").field(arg0).finish(),
            Self::Thunk(_) => write!(f, "thunk"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Listener<'a, T> {
    event: Event<'a>,
    action: Action<'a, T>,
    next: Option<Event<'a>>,
}

impl<'a, T> Default for Listener<'a, T> {
    fn default() -> Self {
        Self {
            event: Default::default(),
            action: Action::NoOp,
            next: Default::default(),
        }
    }
}

impl<'a, T> Listener<'a, T> {
    pub fn dispatch(&mut self, msg: &'static str, transition_expr: &'static str) {
        self.action = Action::Dispatch(msg);

        let mut lexer = Lifecycle::lexer(transition_expr);
        if let Some(Lifecycle::Event(e)) = lexer.next() {
            self.next = Some(e);
        }
    }

    pub fn update(&mut self, thunk: fn(&T, Option<Event<'a>>) -> (T, &'a str)) {
        self.action = Action::Thunk(thunk);
    }
}

#[derive(Debug, Default)]
pub struct Runtime<'a, T> {
    listeners: Vec<Listener<'a, T>>,
    state: Option<T>,
    current: Option<Event<'a>>,
}

pub trait RuntimeState<'a> {
    type Error;
    type State: Sized;

    fn next(&self, msg: &'a str) -> Result<Self::State, Self::Error>;

    fn select(&self, listener: &Listener<'a, Self::State>, current: &Event<'a>) -> bool {
        listener.event.get_phase_lifecycle() == current.get_phase_lifecycle()
            && listener.event.get_prefix_label() == current.get_prefix_label()
            && listener.event.get_payload() == current.get_payload()
    }
}

impl<'a, T> Runtime<'a, T>
where
    T: RuntimeState<'a, State = T> + Default + Clone,
{
    pub fn current(&self) -> Event<'a> {
        self.current.clone().unwrap_or(Event::exit())
    }

    pub fn init() -> T {
        T::default()
    }

    pub fn on(&mut self, event_expr: &'static str) -> &mut Listener<'a, T> {
        let mut lexer = Lifecycle::lexer(event_expr);

        if let Some(Lifecycle::Event(e)) = lexer.next() {
            let listener = Listener {
                event: e,
                action: Action::NoOp,
                next: None,
            };

            self.listeners.push(listener);
            self.listeners.last_mut().unwrap()
        } else {
            panic!()
        }
    }

    pub fn start(&mut self, init_expr: &'a str) {
        let mut lexer = Lifecycle::lexer(init_expr);

        if let Some(Lifecycle::Event(e)) = lexer.next() {
            self.current = Some(e)
        }

        let mut processing = self.process();

        loop {
            processing = processing.process();

            let current = &processing.current();
            if let (_, "exit") = current.get_phase_lifecycle() {
                break;
            }
        }
    }

    fn process(&mut self) -> Self {
        println!("{:?}", &self.current());
        let mut state = self.state.clone();

        match &state {
            None => state = Some(Self::init()),
            _ => {}
        };

        let state = state.unwrap();

        if let Some(l) = &self.listeners
            .iter()
            .find(|l| state.select(l, &self.current()))
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
                    if let Ok(n) = state.next(msg) {
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
}
