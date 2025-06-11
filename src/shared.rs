use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// 扑克牌花色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Suit {
    Hearts,   // 红心
    Diamonds, // 方块
    Clubs,    // 梅花
    Spades,   // 黑桃
}

impl fmt::Display for Suit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Suit::Hearts => write!(f, "♥"),
            Suit::Diamonds => write!(f, "♦"),
            Suit::Clubs => write!(f, "♣"),
            Suit::Spades => write!(f, "♠"),
        }
    }
}

/// 扑克牌点数
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Rank {
    Two = 2,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,  // J
    Queen, // Q
    King,  // K
    Ace,   // A
}

impl Rank {
    pub fn value(&self) -> u8 {
        match self {
            Rank::Two => 2,
            Rank::Three => 3,
            Rank::Four => 4,
            Rank::Five => 5,
            Rank::Six => 6,
            Rank::Seven => 7,
            Rank::Eight => 8,
            Rank::Nine => 9,
            Rank::Ten => 10,
            Rank::Jack => 11,
            Rank::Queen => 12,
            Rank::King => 13,
            Rank::Ace => 14,
        }
    }

    pub fn from_value(value: u8) -> Self {
        match value {
            2 => Rank::Two,
            3 => Rank::Three,
            4 => Rank::Four,
            5 => Rank::Five,
            6 => Rank::Six,
            7 => Rank::Seven,
            8 => Rank::Eight,
            9 => Rank::Nine,
            10 => Rank::Ten,
            11 => Rank::Jack,
            12 => Rank::Queen,
            13 => Rank::King,
            14 | 1 => Rank::Ace, // Ace 表示 14 或 1
            _ => panic!("Invalid value for Rank: {}", value),
        }
    }
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Rank::Two => write!(f, "2"),
            Rank::Three => write!(f, "3"),
            Rank::Four => write!(f, "4"),
            Rank::Five => write!(f, "5"),
            Rank::Six => write!(f, "6"),
            Rank::Seven => write!(f, "7"),
            Rank::Eight => write!(f, "8"),
            Rank::Nine => write!(f, "9"),
            Rank::Ten => write!(f, "10"),
            Rank::Jack => write!(f, "J"),
            Rank::Queen => write!(f, "Q"),
            Rank::King => write!(f, "K"),
            Rank::Ace => write!(f, "A"),
        }
    }
}

/// 单张扑克牌
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.rank, self.suit)
    }
}

/// 玩家行动
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlayerAction {
    Fold,       // 弃牌
    Check,      // 过牌
    Bet(u32),   // 下注
    Raise(u32), // 加注
    Call,       // 跟注
}

/// 游戏阶段
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GameStage {
    PreFlop,  // 翻牌前
    Flop,     // 翻牌圈
    Turn,     // 转牌圈
    River,    // 河牌圈
    Showdown, // 摊牌
}

/// 玩家信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub chips: u32,
    pub cards: Option<(Card, Card)>, // 两张底牌
    pub is_active: bool,             // 是否还在游戏中
    pub current_bet: u32,            // 当前轮已下注额
    pub has_acted: bool,             // 是否已行动
}

/// 游戏状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub players: Vec<Player>,
    pub community_cards: Vec<Card>,
    pub pot: u32,                    // 底池总额
    pub current_player_index: usize, // 当前行动玩家索引
    pub stage: GameStage,
    pub dealer_position: usize, // 庄家位置
    pub small_blind: u32,
    pub big_blind: u32,
}

/// 错误类型
#[derive(Error, Debug, PartialEq)]
pub enum GameError {
    #[error("Invalid player action")]
    InvalidAction,
    #[error("Not enough chips")]
    InsufficientChips,
    #[error("Game stage error")]
    StageError,
    #[error("Player not found")]
    PlayerNotFound,
}
