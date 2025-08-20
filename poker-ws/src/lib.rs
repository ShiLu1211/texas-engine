use rand::seq::SliceRandom;
use std::fmt::{Display, Formatter};
use serde::{Serialize, Deserialize};

pub mod game;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Suit { S, H, D, C }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Rank { Two=2, Three, Four, Five, Six, Seven, Eight, Nine, Ten, Jack, Queen, King, Ace }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card(pub Rank, pub Suit);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deck(pub Vec<Card>);

impl Deck {
    pub fn new() -> Self {
        use Rank::*; use Suit::*;
        let mut cards = Vec::with_capacity(52);
        let ranks = [Two,Three,Four,Five,Six,Seven,Eight,Nine,Ten,Jack,Queen,King,Ace];
        let suits = [S,H,D,C];
        for &s in &suits { for &r in &ranks { cards.push(Card(r,s)); } }
        Self(cards)
    }
    pub fn shuffle(&mut self) {
        let mut rng = rand::thread_rng();
        self.0.as_mut_slice().shuffle(&mut rng);
    }
    pub fn deal(&mut self) -> Option<Card> { self.0.pop() }
    pub fn deal_n(&mut self, n: usize) -> Vec<Card> { (0..n).filter_map(|_| self.deal()).collect() }
}

impl Display for Suit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self { Suit::S => 's', Suit::H => 'h', Suit::D => 'd', Suit::C => 'c' })
    }
}

impl Display for Rank {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use Rank::*;
        let s = match self { Two=>'2',Three=>'3',Four=>'4',Five=>'5',Six=>'6',Seven=>'7',Eight=>'8',Nine=>'9',Ten=>'T',Jack=>'J',Queen=>'Q',King=>'K',Ace=>'A' };
        write!(f, "{}", s)
    }
}

impl Display for Card {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { write!(f, "{}{}", self.0, self.1) }
}

pub fn parse_card(s: &str) -> Card {
    assert!(s.len() == 2, "card like As, Td");
    let bytes = s.as_bytes();
    let r = match bytes[0] as char {
        '2' => Rank::Two,
        '3' => Rank::Three,
        '4' => Rank::Four,
        '5' => Rank::Five,
        '6' => Rank::Six,
        '7' => Rank::Seven,
        '8' => Rank::Eight,
        '9' => Rank::Nine,
        'T' | 't' => Rank::Ten,
        'J' | 'j' => Rank::Jack,
        'Q' | 'q' => Rank::Queen,
        'K' | 'k' => Rank::King,
        'A' | 'a' => Rank::Ace,
        _ => panic!("bad rank"),
    };
    let s = match bytes[1] as char {
        's' | 'S' => Suit::S,
        'h' | 'H' => Suit::H,
        'd' | 'D' => Suit::D,
        'c' | 'C' => Suit::C,
        _ => panic!("bad suit"),
    };
    Card(r, s)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerAction { Fold, Check, Call(u64), Raise(u64) }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Street { Preflop, Flop, Turn, River, Showdown }

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TableState {
    pub small_blind: u64,
    pub big_blind: u64,
    pub pot: u64,
    pub street: Option<Street>,
}

impl TableState {
    pub fn start_hand(&mut self) {
        self.pot = 0;
        self.street = Some(Street::Preflop);
    }
    pub fn next_street(&mut self) {
        self.street = match self.street {
            Some(Street::Preflop) => Some(Street::Flop),
            Some(Street::Flop) => Some(Street::Turn),
            Some(Street::Turn) => Some(Street::River),
            Some(Street::River) => Some(Street::Showdown),
            _ => self.street,
        };
    }
}


