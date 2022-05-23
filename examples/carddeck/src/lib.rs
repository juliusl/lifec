use imgui::Ui;
use lifec::editor::{App, Section, SectionAttributes};
use lifec::RuntimeState;
use logos::{Lexer, Logos};
use rand::seq::SliceRandom;
use rand::thread_rng;
use specs::storage::HashMapStorage;
use specs::Component;
use std::collections::BTreeSet;
use std::fmt::{Debug, Display, Error};

fn from_card(lex: &mut Lexer<Card>) -> Option<(Suit, Value)> {
    let slice = lex.slice();

    let mut value = Value::lexer(&slice[1..]);
    let value = value.next().unwrap();

    let mut suit = Suit::lexer(slice);
    let suit = suit.next().unwrap();

    Some((suit, value))
}

fn from_face_value(lex: &mut Lexer<Value>) -> Option<Face> {
    let mut face = Face::lexer(lex.slice());
    face.next()
}

fn from_number_value(lex: &mut Lexer<Value>) -> Option<u8> {
    let number = lex.slice();
    number.parse().ok()
}

#[derive(Logos, Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub enum Card {
    #[regex("[SsCcDdHh](?:1|2|3|4|5|6|7|8|9|10|[Jj]|[Qq]|[Kk]|[Aa])", from_card)]
    Card((Suit, Value)),

    // Logos requires one token variant to handle errors,
    // it can be named anything you wish.
    #[error]
    // We can also use this variant to define whitespace,
    // or any other matches we wish to skip.
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

#[derive(Logos, Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub enum Suit {
    #[regex("[Ss]")]
    Spade,
    #[regex("[Cc]")]
    Clover,
    #[regex("[Dd]")]
    Diamond,
    #[regex("[Hh]")]
    Heart,
    // Logos requires one token variant to handle errors,
    // it can be named anything you wish.
    #[error]
    // We can also use this variant to define whitespace,
    // or any other matches we wish to skip.
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

#[derive(Logos, Hash, Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum Value {
    #[regex("1|2|3|4|5|6|7|8|9|10", from_number_value)]
    Number(u8),

    #[regex("[Jj]|[Qq]|[Kk]|[Aa]", from_face_value)]
    Face(Face),
    // Logos requires one token variant to handle errors,
    // it can be named anything you wish.
    #[error]
    // We can also use this variant to define whitespace,
    // or any other matches we wish to skip.
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

#[derive(Logos, Hash, Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum Face {
    #[regex("[Jj]")]
    Jack,
    #[regex("[Qq]")]
    Queen,
    #[regex("[Kk]")]
    King,
    #[regex("[Aa]")]
    Ace,
    // Logos requires one token variant to handle errors,
    // it can be named anything you wish.
    #[error]
    // We can also use this variant to define whitespace,
    // or any other matches we wish to skip.
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

impl Card {
    pub fn card(suit: Suit, value: Value) -> Card {
        Card::Card((suit, value))
    }

    pub fn heart<T: Into<Value> + Sized>(value: T) -> Card {
        Card::Card((Suit::Heart, value.into()))
    }

    pub fn diamond<T: Into<Value> + Sized>(value: T) -> Card {
        Card::Card((Suit::Diamond, value.into()))
    }

    pub fn clover<T: Into<Value> + Sized>(value: T) -> Card {
        Card::Card((Suit::Clover, value.into()))
    }

    pub fn spade<T: Into<Value> + Sized>(value: T) -> Card {
        Card::Card((Suit::Spade, value.into()))
    }

    pub fn suit(&self) -> &Suit {
        if let Card::Card((s, _)) = self {
            return s;
        } else {
            unreachable!()
        }
    }

    pub fn value(&self) -> &Value {
        if let Card::Card((_, v)) = self {
            return v;
        } else {
            unreachable!()
        }
    }
}

impl Display for Suit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Suit::Spade => write!(f, "s"),
            Suit::Clover => write!(f, "c"),
            Suit::Diamond => write!(f, "d"),
            Suit::Heart => write!(f, "h"),
            Suit::Error => Err(Error {}),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(i) => write!(f, "{}", i),
            Value::Face(face) => match face {
                Face::Jack => write!(f, "j"),
                Face::Queen => write!(f, "q"),
                Face::King => write!(f, "k"),
                Face::Ace => write!(f, "a"),
                Face::Error => Err(Error {}),
            },
            Value::Error => Err(Error {}),
        }
    }
}

impl Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let suit = self.suit();
        let value = self.value();

        std::fmt::Display::fmt(suit, f)?;
        std::fmt::Display::fmt(value, f)?;

        Ok(())
    }
}

impl Into<u8> for &Value {
    fn into(self) -> u8 {
        match self {
            Value::Number(v) => *v,
            Value::Face(f) => match f {
                Face::Jack => 11,
                Face::Queen => 12,
                Face::King => 13,
                Face::Ace => 14,
                _ => 0,
            },
            _ => 0,
        }
    }
}

impl From<u8> for Value {
    fn from(v: u8) -> Self {
        if v >= 1 && v < 11 {
            Value::Number(v)
        } else if v >= 11 {
            match v {
                11 => Value::from(Face::Jack),
                12 => Value::from(Face::Queen),
                13 => Value::from(Face::King),
                14 => Value::from(Face::Ace),
                _ => Value::Error,
            }
        } else {
            Value::Error
        }
    }
}

impl From<Face> for Value {
    fn from(v: Face) -> Self {
        Value::Face(v)
    }
}

#[test]
fn test_types() {
    assert!(Value::Number(10) < Value::Face(Face::Jack));
    assert!(Face::Jack < Face::Ace);
    assert!(Suit::Spade < Suit::Heart);
    assert!(&Suit::Spade < &Suit::Heart);
    assert!(&Suit::Heart > &Suit::Spade);

    assert!(Card::heart(Face::Ace) > Card::diamond(Face::Ace));
    assert!(Card::spade(1) == Card::spade(1));
    assert!(Card::heart(Face::Ace) > Card::heart(10));
    assert!(Card::heart(Face::Ace) > Card::heart(Face::Jack));
    assert!(Card::heart(Face::Ace) != Card::heart(1));
    assert!(Card::heart(Face::Jack) == Card::heart(11));
    assert!(Card::heart(Face::Queen) == Card::heart(12));
    assert!(Card::heart(Face::King) == Card::heart(13));
}

#[test]
fn test_lexer() {
    let mut lex = Card::lexer("h5dAs10c5");

    assert_eq!(Some(Card::heart(5)), lex.next());
    assert_eq!(Some(Card::diamond(Face::Ace)), lex.next());
    assert_eq!(Some(Card::spade(10)), lex.next());
    assert_eq!(Some(Card::clover(5)), lex.next());
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hand(Vec<Card>);

pub struct EmptyHandError {}

impl From<&str> for Hand {
    fn from(value: &str) -> Self {
        let mut cards: Vec<Card> = vec![];
        let mut lex = Card::lexer(value);

        loop {
            match lex.next() {
                Some(Card::Card(c)) => cards.push(Card::Card(c)),
                Some(Card::Error) => continue,
                None => break,
            }
        }

        Hand(cards)
    }
}

impl Hand {
    pub fn sort(self) -> Self {
        let mut v = self.0;
        v.sort();
        Self(v)
    }

    pub fn shuffle(self) -> Self {
        let mut v = self.0;
        let mut rng = thread_rng();
        v.shuffle(&mut rng);
        Self(v)
    }

    pub fn take(&self, n: usize) -> Option<(Self, Vec<Card>)> {
        if n > self.0.len() {
            return None;
        }

        let v: Vec<Card> = self.0.iter().cloned().take(n).collect();
        let remaining: Vec<Card> = self.0.iter().cloned().skip(n).collect();

        Some((Self(remaining), v))
    }

    pub fn pick(&self, picks: &[usize]) -> Option<(Self, Vec<Card>)> {
        let picks: Vec<Card> = picks
            .iter()
            .filter_map(|i| self.0.get(*i))
            .cloned()
            .collect();

        let rem: Vec<Card> = self
            .0
            .iter()
            .filter(|f| !picks.contains(*f))
            .cloned()
            .collect();

        Some((Self(picks), rem))
    }

    pub fn suits(&self) -> Vec<&Suit> {
        self.0.iter().map(Card::suit).collect()
    }

    pub fn values(&self) -> Vec<&Value> {
        self.0.iter().map(Card::value).collect()
    }

    pub fn values_u8(&self) -> Vec<u8> {
        self.values()
            .iter()
            .map::<u8, _>(|f| f.to_owned().into())
            .collect()
    }

    pub fn cards(&self) -> Vec<&Card> {
        self.0.iter().collect()
    }

    pub fn count_hearts(&self) -> usize {
        self.count_suit(Suit::Heart)
    }

    pub fn count_diamonds(&self) -> usize {
        self.count_suit(Suit::Diamond)
    }

    pub fn count_clovers(&self) -> usize {
        self.count_suit(Suit::Clover)
    }

    pub fn count_spades(&self) -> usize {
        self.count_suit(Suit::Spade)
    }

    pub fn count_values(&self, value: Value) -> usize {
        self.0
            .iter()
            .map(Card::value)
            .filter(|f| **f == value)
            .count()
    }

    pub fn count_suit(&self, suit: Suit) -> usize {
        self.0
            .iter()
            .map(Card::suit)
            .filter(|f| **f == suit)
            .count()
    }

    pub fn is_straight(&self) -> bool {
        // Exit early if we know this entire hand cannot be a straight

        // If there are duplicate cards, then the len of the set would not match;
        // the len of values
        if BTreeSet::from_iter(self.values()).len() != self.0.len() {
            return false;
        }

        // Compare the sum of values with the case all numbers were consecutive
        let values = self.values();

        // sum of consecutive numbers
        // Sum of m to n
        // n(n+1)/2 - m(m-1)/2
        let min: u8 = values.iter().min().unwrap().to_owned().into();
        let max: u8 = values.iter().max().unwrap().to_owned().into();
        let n = (max * (max + 1)) / 2;
        let m = (min * (min - 1)) / 2;
        let expected = n - m;

        // actual sum of values
        let values: u8 = self.values_u8().iter().sum();

        expected == values
    }

    pub fn as_straight(&self) -> Option<(u8, u8)> {
        if self.is_straight() {
            Some((
                self.values().iter().min().unwrap().to_owned().into(),
                self.values().iter().max().unwrap().to_owned().into(),
            ))
        } else {
            None
        }
    }

    pub fn is_flush(&self) -> bool {
        self.count_clovers() == self.0.len()
            || self.count_hearts() == self.0.len()
            || self.count_diamonds() == self.0.len()
            || self.count_spades() == self.0.len()
    }

    pub fn as_flush(&self) -> Option<&Suit> {
        if self.is_flush() {
            self.0.first().and_then(|f| Some(f.suit()))
        } else {
            None
        }
    }
}

#[test]
fn test_hand() {
    assert!(Hand::try_from("h5dAs10c5ca").is_ok());

    if let Ok(hand) = Hand::try_from("h5 ha h2 h3 hq") {
        let cards = hand.cards();
        assert_eq!(*cards[0], Card::heart(5));
        assert_eq!(*cards[1], Card::heart(14));
        assert_eq!(*cards[2], Card::heart(2));
        assert_eq!(*cards[3], Card::heart(3));
        assert_eq!(*cards[4], Card::heart(12));

        let values = hand.values();
        assert_eq!(*values[0], Value::from(5));
        assert_eq!(*values[1], Value::from(14));
        assert_eq!(*values[2], Value::from(2));
        assert_eq!(*values[3], Value::from(3));
        assert_eq!(*values[4], Value::from(12));

        assert_eq!(hand.count_hearts(), 5);
    }

    if let Ok(hand) = Hand::try_from("h5 s5 d5 c5") {
        assert_eq!(hand.count_values(Value::from(5)), 4);
    }

    if let Ok(hand) = Hand::try_from("h1 h2 h3 h4 h5") {
        assert!(hand.is_straight());
        assert!(hand.is_flush());
        assert_eq!(Some((1, 5)), hand.as_straight());
    }

    if let Ok(hand) = Hand::try_from("h9 h10 hJ hq") {
        assert!(hand.is_straight());
        assert!(hand.is_flush());
        assert_eq!(Some((9, 12)), hand.as_straight());
    }

    if let Ok(hand) = Hand::try_from("h10 h9 hJ hq hk ha") {
        assert!(hand.is_straight());
        assert!(hand.is_flush());
        assert_eq!(Some((9, 14)), hand.as_straight());
    }

    if let Ok(hand) = Hand::try_from("h10 h9 hq hJ") {
        assert!(hand.is_straight());
        assert!(hand.is_flush());
        assert_eq!(Some((9, 12)), hand.as_straight());
    }

    if let Ok(hand) = Hand::try_from("s10 d9 hq cJ hk sa") {
        assert!(hand.is_straight());
    }

    if let Ok(hand) = Hand::try_from("s7 d7 h10 cJ h10") {
        assert!(!hand.is_straight());
    }

    if let Ok(hand) = Hand::try_from("s4 s4 d1 h1") {
        assert!(!hand.is_straight());
    }
}

fn from_hand(lex: &mut Lexer<Deck>) -> Option<Hand> {
    if let Ok(v) = Hand::try_from(lex.slice()) {
        Some(v)
    } else {
        None
    }
}

const ALL_SPADES: &str = r"s2s3s4s5s6s7s8s9s10sjsqsksa";
const ALL_CLOVERS: &str = r"c2c3c4c5c6c7c8c9c10cjcqckca";
const ALL_DIAMONDS: &str = r"d2d3d4d5d6d7d8d9d10djdqdkda";
const ALL_HEARTS: &str = r"h2h3h4h5h6h7h8h9h10hjhqhkha";

fn from_standard_deck(_: &mut Lexer<Deck>) -> Option<Hand> {
    let standard_deck = format!(
        "{}{}{}{}",
        ALL_SPADES, ALL_CLOVERS, ALL_DIAMONDS, ALL_HEARTS
    );

    if let Ok(v) = Hand::try_from(standard_deck.as_str()) {
        Some(v.shuffle())
    } else {
        None
    }
}

const ALL_SPADES_ACE_LOW: &str = r"s1s2s3s4s5s6s7s8s9s10sjsqsk";
const ALL_CLOVERS_ACE_LOW: &str = r"c1c2c3c4c5c6c7c8c9c10cjcqck";
const ALL_DIAMONDS_ACE_LOW: &str = r"d1d2d3d4d5d6d7d8d9d10djdqdk";
const ALL_HEARTS_ACE_LOW: &str = r"h1h2h3h4h5h6h7h8h9h10hjhqhk";

fn from_standard_deck_ace_low(_: &mut Lexer<Deck>) -> Option<Hand> {
    let standard_deck = format!(
        "{}{}{}{}",
        ALL_SPADES_ACE_LOW, ALL_CLOVERS_ACE_LOW, ALL_DIAMONDS_ACE_LOW, ALL_HEARTS_ACE_LOW
    );

    if let Ok(v) = Hand::try_from(standard_deck.as_str()) {
        Some(v.shuffle())
    } else {
        None
    }
}

#[derive(Logos, Debug, Clone, PartialEq)]
pub enum Deck {
    #[regex(r"\(.STD_ACE_LOW\)"r, from_standard_deck_ace_low)]
    #[regex(r"\(.STD\)"r, from_standard_deck)]
    #[regex(r"\((:?[SsCcDdHh](?:1|2|3|4|5|6|7|8|9|10|[Jj]|[Qq]|[Kk]|[Aa]))*\)"r, from_hand)]
    Deck(Hand),
    #[regex(r"\[(:?[SsCcDdHh](?:1|2|3|4|5|6|7|8|9|10|[Jj]|[Qq]|[Kk]|[Aa]))*\]"r, from_hand)]
    Hand(Hand),
    #[token(r"\[\]")]
    EmptyHand,
    // Logos requires one token variant to handle errors,
    // it can be named anything you wish.
    #[error]
    // We can also use this variant to define whitespace,
    // or any other matches we wish to skip.
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

impl Deck {
    pub fn shuffle(self) -> Self {
        match self {
            Deck::Deck(h) => Deck::Deck(h.shuffle()),
            Deck::Hand(h) => Deck::Hand(h.shuffle()),
            _ => self,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Deck::Deck(d) => d.0.len() <= 0,
            Deck::Hand(d) => d.0.len() <= 0,
            _ => true,
        }
    }
}

impl Display for Hand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for c in self.cards().iter() {
            std::fmt::Display::fmt(*c, f)?;
        }

        Ok(())
    }
}

impl Display for Deck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Deck::Deck(h) => {
                write!(f, "(")?;
                std::fmt::Display::fmt(h, f)?;
                write!(f, ")")?;
                Ok(())
            }
            Deck::Hand(h) => {
                write!(f, "[")?;
                std::fmt::Display::fmt(h, f)?;
                write!(f, "]")?;
                Ok(())
            }
            Deck::EmptyHand => {
                write!(f, "[")?;
                write!(f, "]")?;
                Ok(())
            }
            Deck::Error => Err(Error {}),
        }
    }
}

#[test]
fn test_deck() {
    let mut deck = Deck::lexer("[h1h2h3h4h5][d1d2d4d6](s1s2s3)(.STD)(.STD_ACE_LOW)");

    if let Ok(hand) = Hand::try_from("h1 h2 h3 h4 h5") {
        assert_eq!(Some(Deck::Hand(hand)), deck.next());
    }

    if let Ok(hand) = Hand::try_from("d2 d1 d4 d6") {
        assert_eq!(Some(Deck::Hand(hand.sort())), deck.next());
    }

    if let Ok(hand) = Hand::try_from("s1 s2 s3") {
        let deck = deck.next();
        assert_eq!(Some(Deck::Deck(hand)), deck);
    }

    if let Some(Deck::Deck(h)) = deck.next() {
        assert_eq!(52, h.cards().len());
        assert_eq!(13, h.count_spades());
        assert_eq!(13, h.count_clovers());
        assert_eq!(13, h.count_diamonds());
        assert_eq!(13, h.count_hearts());
        (2..14).for_each(|f| assert_eq!(4, h.count_values(Value::from(f))));

        if let Some((remaining, taken)) = h.take(13) {
            println!("standard_deck");
            println!("remaining: {}", Deck::Deck(remaining.sort()));
            println!("hand:      {}", Deck::Hand(Hand(taken).sort()));
        }
    } else {
        assert!(false, "expected a shuffled standard playing deck");
    }

    if let Some(Deck::Deck(h)) = deck.next() {
        assert_eq!(52, h.cards().len());
        assert_eq!(13, h.count_spades());
        assert_eq!(13, h.count_clovers());
        assert_eq!(13, h.count_diamonds());
        assert_eq!(13, h.count_hearts());
        (1..13).for_each(|f| assert_eq!(4, h.count_values(Value::from(f))));

        if let Some((remaining, taken)) = h.take(13) {
            println!("standard_deck_ace_low");
            println!("remaining: {}", Deck::Deck(remaining.sort()));
            println!("hand:      {}", Deck::Hand(Hand(taken).sort()));
        }
    } else {
        assert!(false, "expected a shuffled standard playing deck");
    }
}

fn from_draw(l: &mut Lexer<Draw>) -> Option<usize> {
    let count = &l.slice()[2..l.slice().len() - 1];
    count.parse().ok()
}

fn from_draw_to(l: &mut Lexer<Draw>) -> Option<(usize, usize)> {
    let count_start = l.slice().chars().position(|p| p == '+').unwrap();
    let player = &l.slice()[2..count_start];
    let player: usize = player.parse().ok().unwrap();
    let count = &l.slice()[count_start + 1..l.slice().len() - 1];
    let count: usize = count.parse().ok().unwrap();

    Some((player, count))
}

fn from_take_from(l: &mut Lexer<Draw>) -> Option<(usize, usize)> {
    let count_start = l.slice().chars().position(|p| p == '-').unwrap();
    let player = &l.slice()[2..count_start];
    let player: usize = player.parse().ok().unwrap();
    let count = &l.slice()[count_start + 1..l.slice().len() - 1];
    let count: usize = count.parse().ok().unwrap();

    Some((player, count))
}

#[derive(Logos, Debug, PartialEq)]
pub enum Draw {
    #[regex(r"\[[+][0-9]+\]"r, from_draw)]
    Draw(usize),
    #[regex(r"\[[.][0-9]+[+][0-9]+\]"r, from_draw_to)]
    DrawTo((usize, usize)),
    #[regex(r"\[[.][0-9]+[-][0-9]+\]"r, from_take_from)]
    TakeFrom((usize, usize)),
    // Logos requires one token variant to handle errors,
    // it can be named anything you wish.
    #[error]
    // We can also use this variant to define whitespace,
    // or any other matches we wish to skip.
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

#[test]
fn test_draw() {
    let mut draw = Draw::lexer("[+4][+5][+6]");
    assert_eq!(Some(Draw::Draw(4)), draw.next());
    assert_eq!(Some(Draw::Draw(5)), draw.next());
    assert_eq!(Some(Draw::Draw(6)), draw.next());
}

#[derive(Debug, Clone, Component)]
#[storage(HashMapStorage)]
pub struct Dealer {
    deck: Deck,
    hands: Vec<Hand>,
    expression: String,
}

impl Dealer {
    pub fn dealer_section() -> Section<Dealer> {
        Dealer::default().into()
    }
}

#[derive(Debug)]
pub struct InvalidDealerExpression {}

impl TryFrom<&str> for Dealer {
    type Error = InvalidDealerExpression;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut deck_l = Deck::lexer(value);
        let mut hands = vec![];

        let _deck: Deck;
        loop {
            match deck_l.next() {
                Some(Deck::Hand(h)) => {
                    hands.push(h);
                    continue;
                }
                Some(Deck::Deck(h)) => {
                    _deck = Deck::Deck(h);
                    break;
                }
                Some(Deck::EmptyHand) => {
                    hands.push(Hand(vec![]));
                    continue;
                }
                None => {
                    _deck = Deck::Deck(Hand(vec![]));
                    break;
                }
                _ => continue,
            };
        }

        if let Deck::Deck(d) = _deck {
            let mut draws = Draw::lexer(value);
            let mut deck = d.clone();

            loop {
                let remaining = match draws.next() {
                    Some(Draw::Draw(count)) => {
                        if let Some((remaining, new_hand)) = deck.to_owned().shuffle().take(count) {
                            hands.push(Hand(new_hand));
                            Some(remaining)
                        } else {
                            None
                        }
                    }
                    Some(Draw::DrawTo((player_pos, count))) => {
                        if let Some(hand) = hands.get(player_pos) {
                            if let Some((remaining, mut new_hand)) =
                                deck.to_owned().shuffle().take(count)
                            {
                                let next_hand = hand.0.clone();
                                let mut next_hand = next_hand;
                                next_hand.append(&mut new_hand);
                                hands[player_pos] = Hand(next_hand);
                                Some(remaining)
                            } else {
                                None
                            }
                        } else {
                            eprintln!("draw_to, invalid player_pos skipping, {}", player_pos);
                            continue;
                        }
                    }
                    Some(Draw::TakeFrom((player_pos, count))) => {
                        if let Some(hand) = hands.get(player_pos) {
                            if let Some((remaining, mut new_hand)) =
                                hand.to_owned().shuffle().take(count)
                            {
                                let next_hand = deck.0.clone();
                                let mut next_hand = next_hand;
                                next_hand.append(&mut new_hand);
                                hands[player_pos] = remaining;
                                Some(Hand(next_hand))
                            } else {
                                None
                            }
                        } else {
                            eprintln!("take_from, invalid player_pos skipping. {}", player_pos);
                            continue;
                        }
                    }
                    Some(Draw::Error) => continue,
                    None => break,
                };

                match remaining {
                    Some(h) => deck = h,
                    None => break,
                }
            }

            let deck = Deck::Deck(deck);
            Ok(Self {
                deck,
                hands,
                expression: format!(""),
            })
        } else {
            Err(InvalidDealerExpression {})
        }
    }
}

impl Display for Dealer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for hand in self.hands.iter() {
            std::fmt::Display::fmt(&Deck::Hand(hand.clone()), f)?;
        }

        std::fmt::Display::fmt(&self.deck, f)?;

        Ok(())
    }
}

impl Default for Dealer {
    fn default() -> Self {
        let dealer = Dealer::try_from("(.STD)").ok();
        dealer.unwrap()
    }
}

impl From<&Hand> for Dealer {
    fn from(hand: &Hand) -> Self {
        let h = Self {
            deck: Deck::Deck(hand.clone()),
            hands: vec![],
            expression: String::default(),
        };

        if let Ok(s) = h.deal("[+1]".repeat(hand.0.len()).as_str()) {
            s
        } else {
            Self {
                deck: Deck::Deck(Hand(vec![])),
                hands: vec![],
                expression: String::default(),
            }
        }
    }
}

impl App for Dealer {
    fn name() -> &'static str {
        "Card Dealer"
    }

    fn show_editor(&mut self, ui: &Ui) {
        ui.text(format!("{:?}", self.into_attributes()));
        ui.indent();
        ui.label_text("Number of hands", format!("{}", self.hands()));

        for i in 0..self.hands() {
            if let Some(hand) = self.hand(i) {
                ui.label_text(format!("Hand: {}", i), format!("{}", hand));
            }
        }

        if let Some(deck) = self.deck() {
            ui.label_text("Deck", format!("{}", deck));
        }

        ui.input_text(format!("Expression"), &mut self.expression)
            .build();
        if ui.button(format!("Deal")) {
            match self.deal(&self.expression) {
                Ok(next) => *self = next,
                Err(e) => eprintln!("Error Dealing: {:?}", e),
            }
        }
        ui.same_line();
        ui.text("Parses the above expression and updates state");

        if ui.button("Reshuffle") {
            *self = Dealer::default();
        }
        ui.same_line();
        ui.text("Reshuffles the deck and all hands");

        if ui.button("Clear Deck") {
            *self = self.clear_deck();
        }
        ui.same_line();
        ui.text("Clears the deck");

        if ui.button("Prune") {
            *self = self.prune()
        }
        ui.same_line();
        ui.text("Removes empty hands");
        ui.unindent();
    }
}

impl RuntimeState for Dealer {
    type Error = InvalidDealerExpression;

    fn load<S: AsRef<str> + ?Sized>(&self, init: &S) -> Self
    where
        Self: Sized,
    {
        if let Ok(dealer) = Dealer::try_from(init.as_ref()) {
            dealer
        } else {
            panic!("could not parse {}", init.as_ref())
        }
    }

    fn process<S: AsRef<str> + ?Sized>(&self, msg: &S) -> Result<Self, Self::Error> {
        println!("Received: {}", msg.as_ref());
        self.deal(msg.as_ref())
    }

    fn process_with_args<S: AsRef<str> + ?Sized>(
        state: lifec::WithArgs<Self>,
        msg: &S,
    ) -> Result<Self, Self::Error>
    where
        Self: Clone + Default + RuntimeState,
    {
        let args = state.parse_flags();

        println!("Dealer received args: {:?}", args);

        state.get_state().deal(msg.as_ref())
    }

    fn from_attributes(attributes: Vec<lifec::editor::Attribute>) -> Self {
        if let Some(lifec::editor::Value::TextBuffer(s)) = SectionAttributes::from(attributes).get_attr_value("carddeck::") {
            Self::default().load(s)
        } else {
            Self::default()
        }
    }

    fn into_attributes(&self) -> Vec<lifec::editor::Attribute> {
        SectionAttributes::default()
            .with_text("carddeck::", format!("{}", self))
            .clone_attrs()
    }
}

impl Dealer {
    pub fn deal(&self, draw_expr: &str) -> Result<Self, InvalidDealerExpression> {
        Dealer::try_from(format!("{}{}", self, draw_expr).as_str())
    }

    pub fn hand(&self, pos: usize) -> Option<&Hand> {
        self.hands.get(pos)
    }

    pub fn hands(&self) -> usize {
        self.hands.len()
    }

    pub fn deck(&self) -> Option<&Hand> {
        match &self.deck {
            Deck::Deck(h) => Some(h),
            _ => None,
        }
    }

    pub fn prune(&self) -> Self {
        let hands: Vec<Hand> = self
            .hands
            .iter()
            .filter(|h| h.0.len() > 0)
            .cloned()
            .collect();

        Self {
            hands,
            deck: self.deck.clone(),
            expression: self.expression.clone(),
        }
    }

    pub fn clear_deck(&self) -> Self {
        Self {
            hands: self.hands.iter().cloned().collect(),
            deck: Deck::Deck(Hand(vec![])),
            expression: String::default(),
        }
    }
}

#[test]
fn test_dealer() {
    if let Ok(dealer) = Dealer::try_from("(.STD)[+13][+13][+13][+13]") {
        println!("{}", dealer);
    } else {
        assert!(false, "Expected the dealer expression to be valid");
    }

    if let Ok(dealer) = Dealer::try_from("(.STD)[+3][+2][+2][+2]") {
        println!("{}", dealer);
        let dealer = dealer.deal("[.0+1][.0+1]").ok();
        let dealer = dealer.unwrap();
        println!("{}", dealer);
        let dealer = dealer.deal("[.1+1][.2+1][.3+1][.4+1]").ok();
        let dealer = dealer.unwrap();
        println!("{}", dealer);
        let dealer = dealer.deal("[+38][.2+1][.3+1][.4+1]").ok();
        let dealer = dealer.unwrap();
        println!("{}", dealer);
        let dealer = dealer.deal("[.4-37][.4+1][.4+1][.4-1]").ok();
        let dealer = dealer.unwrap();
        println!("{}", dealer);
        let dealer = dealer.deal("[.1-1][.2-1][.3-1]").ok();
        let dealer = dealer.unwrap();
        println!("{}", dealer);

        println!("{}", dealer.hand(0).unwrap());
        println!("{}", dealer.hand(1).unwrap());
        println!("{}", dealer.hand(2).unwrap());
        println!("{}", dealer.hand(3).unwrap());
        println!("{}", dealer.hand(4).unwrap());

        let player = dealer.hand(0).unwrap().pick(&[1, 2, 3]);
        println!("{:?}", player);

        // Demo: how to select specific cards from a hand
        let dealer = Dealer::from(dealer.hand(0).unwrap());
        println!("{}", dealer);

        // This is like picking 3 out of original 5 cards
        let dealer = dealer.deal("[.1-1][.2-1][.3-1]").ok().unwrap().prune();

        println!("{}", dealer);
        println!("{}", Deck::Hand(dealer.deck().unwrap().to_owned()));

        // This is
        let dealer = dealer
            .clear_deck()
            .deal("[.0-1][.1-1]")
            .ok()
            .unwrap()
            .prune();
        println!("{}", Deck::Hand(dealer.deck().unwrap().to_owned()));
    } else {
        assert!(false, "Expected the dealer expression to be valid");
    }
}
