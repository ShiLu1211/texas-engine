use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use super::shared::*;
use itertools::Itertools;
use rand::rng;
use rand::seq::SliceRandom;

/// 创建一副洗好的牌
pub fn create_shuffled_deck() -> Vec<Card> {
    let mut deck = Vec::new();

    let suits = [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades];
    let ranks = [
        Rank::Two,
        Rank::Three,
        Rank::Four,
        Rank::Five,
        Rank::Six,
        Rank::Seven,
        Rank::Eight,
        Rank::Nine,
        Rank::Ten,
        Rank::Jack,
        Rank::Queen,
        Rank::King,
        Rank::Ace,
    ];

    for &suit in &suits {
        for &rank in &ranks {
            deck.push(Card { suit, rank });
        }
    }

    let mut rng = rng();
    deck.shuffle(&mut rng);
    deck
}

/// 评估玩家手牌强度
pub fn evaluate_hand(player_cards: &(Card, Card), community_cards: &[Card]) -> HandEvaluation {
    // 合并所有牌
    let mut all_cards = vec![player_cards.0, player_cards.1];
    all_cards.extend_from_slice(community_cards);

    // 找出最佳5张牌组合
    let best_hand = find_best_five_card_hand(&all_cards);

    // 评估最佳5张牌组合
    evaluate_five_cards(&best_hand)
}

/// 从所有可用牌中找出最佳的5张牌组合
fn find_best_five_card_hand(cards: &[Card]) -> Vec<Card> {
    // 如果牌数不超过5张，直接返回所有牌
    if cards.len() <= 5 {
        return cards.to_vec();
    }

    // 生成所有可能的5张牌组合
    let mut best_hand = vec![cards[0], cards[1], cards[2], cards[3], cards[4]];
    let mut best_evaluation = evaluate_five_cards(&best_hand);

    // 遍历所有5张牌组合
    for hand in cards.iter().combinations(5) {
        let hand_cloned = hand.into_iter().copied().collect::<Vec<_>>();
        let evaluation = evaluate_five_cards(&hand_cloned);

        // 如果找到更好的牌型，更新最佳组合
        if evaluation.rank > best_evaluation.rank
            || (evaluation.rank == best_evaluation.rank
                && compare_kickers(&evaluation.kickers, &best_evaluation.kickers)
                    == Ordering::Greater)
        {
            best_evaluation = evaluation;
            best_hand = hand_cloned;
        }
    }
    best_hand
}

/// 评估5张牌的牌型
fn evaluate_five_cards(cards: &[Card]) -> HandEvaluation {
    assert!(cards.len() == 5, "只能评估5张牌");

    // 按点数分组
    let mut rank_counts: HashMap<Rank, u8> = HashMap::new();
    for card in cards {
        *rank_counts.entry(card.rank).or_insert(0) += 1;
    }

    // 按花色分组
    let mut suit_counts: HashMap<Suit, u8> = HashMap::new();
    for card in cards {
        *suit_counts.entry(card.suit).or_insert(0) += 1;
    }

    // 检查同花
    let is_flush = suit_counts.values().any(|&count| count == 5);

    // 检查顺子
    let (is_straight, straight_high) = check_straight(cards);

    // 检查皇家同花顺
    if is_flush && is_straight && straight_high == Rank::Ace {
        return HandEvaluation {
            rank: HandRank::RoyalFlush,
            kickers: vec![Rank::Ace],
        };
    }

    // 检查同花顺
    if is_flush && is_straight {
        return HandEvaluation {
            rank: HandRank::StraightFlush,
            kickers: vec![straight_high],
        };
    }

    // 检查四条
    if let Some(quad_rank) = rank_counts
        .iter()
        .find(|&(_, &count)| count == 4)
        .map(|(r, _)| *r)
    {
        let kicker = *rank_counts
            .iter()
            .filter(|(r, _)| **r != quad_rank)
            .map(|(r, _)| r)
            .max()
            .unwrap();

        return HandEvaluation {
            rank: HandRank::FourOfAKind,
            kickers: vec![quad_rank, kicker],
        };
    }

    // 检查葫芦（三条+对子）
    if let Some(three_rank) = rank_counts
        .iter()
        .find(|&(_, &count)| count == 3)
        .map(|(r, _)| *r)
    {
        if let Some(pair_rank) = rank_counts
            .iter()
            .filter(|(r, _)| **r != three_rank)
            .find(|&(_, &count)| count >= 2)
            .map(|(r, _)| *r)
        {
            return HandEvaluation {
                rank: HandRank::FullHouse,
                kickers: vec![three_rank, pair_rank],
            };
        }
    }

    // 检查同花
    if is_flush {
        let mut kickers: Vec<Rank> = cards.iter().map(|c| c.rank).collect();
        kickers.sort_by(|a, b| b.cmp(a)); // 降序排序
        return HandEvaluation {
            rank: HandRank::Flush,
            kickers,
        };
    }

    // 检查顺子
    if is_straight {
        return HandEvaluation {
            rank: HandRank::Straight,
            kickers: vec![straight_high],
        };
    }

    // 检查三条
    if let Some(three_rank) = rank_counts
        .iter()
        .find(|&(_, &count)| count == 3)
        .map(|(r, _)| *r)
    {
        let mut kickers: Vec<Rank> = rank_counts
            .iter()
            .filter(|(r, _)| **r != three_rank)
            .map(|(r, _)| *r)
            .collect();

        kickers.sort_by(|a, b| b.cmp(a)); // 降序排序
        kickers.truncate(2); // 只保留最大的两个踢脚牌

        return HandEvaluation {
            rank: HandRank::ThreeOfAKind,
            kickers: std::iter::once(three_rank).chain(kickers).collect(),
        };
    }

    // 检查两对
    let pairs: Vec<Rank> = rank_counts
        .iter()
        .filter(|&(_, &count)| count == 2)
        .map(|(r, _)| *r)
        .collect();

    if pairs.len() >= 2 {
        let mut sorted_pairs = pairs;
        sorted_pairs.sort_by(|a, b| b.cmp(a)); // 降序排序
        let high_pair = sorted_pairs[0];
        let low_pair = sorted_pairs[1];

        let kicker = *rank_counts
            .iter()
            .filter(|(r, _)| **r != high_pair && **r != low_pair)
            .map(|(r, _)| r)
            .max()
            .unwrap();

        return HandEvaluation {
            rank: HandRank::TwoPair,
            kickers: vec![high_pair, low_pair, kicker],
        };
    }

    // 检查一对
    if let Some(pair_rank) = rank_counts
        .iter()
        .find(|&(_, &count)| count == 2)
        .map(|(r, _)| *r)
    {
        let mut kickers: Vec<Rank> = rank_counts
            .iter()
            .filter(|(r, _)| **r != pair_rank)
            .flat_map(|(r, _)| vec![*r; 1])
            .collect();

        kickers.sort_by(|a, b| b.cmp(a)); // 降序排序
        kickers.truncate(3); // 只保留最大的三个踢脚牌

        return HandEvaluation {
            rank: HandRank::OnePair,
            kickers: std::iter::once(pair_rank).chain(kickers).collect(),
        };
    }

    // 高牌
    let mut kickers: Vec<Rank> = cards.iter().map(|c| c.rank).collect();
    kickers.sort_by(|a, b| b.cmp(a)); // 降序排序
    HandEvaluation {
        rank: HandRank::HighCard,
        kickers,
    }
}

/// 检查是否为顺子并返回最大牌
fn check_straight(cards: &[Card]) -> (bool, Rank) {
    // 用于数值判断的集合
    let mut values: HashSet<u8> = cards.iter().map(|c| c.rank.value()).collect();

    // 特殊处理：A=14 也可以视为 1
    if values.contains(&14) {
        values.insert(1);
    }

    let mut sorted_values: Vec<u8> = values.into_iter().collect();
    sorted_values.sort();

    let mut consecutive = 1;
    let mut max_value = 0;

    for i in 1..sorted_values.len() {
        if sorted_values[i] == sorted_values[i - 1] + 1 {
            consecutive += 1;
            if consecutive >= 5 {
                max_value = sorted_values[i];
            }
        } else if sorted_values[i] != sorted_values[i - 1] {
            consecutive = 1;
        }
    }

    if max_value > 0 {
        return (true, Rank::from_value(max_value));
    }

    (false, Rank::Two)
}

/// 比较两个关键牌序列
pub fn compare_kickers(k1: &[Rank], k2: &[Rank]) -> Ordering {
    for (a, b) in k1.iter().zip(k2.iter()) {
        let cmp = a.cmp(b);
        if cmp != Ordering::Equal {
            return cmp;
        }
    }
    Ordering::Equal
}

/// 比较两手牌
pub fn compare_hands(
    hand1: &(Card, Card),
    hand2: &(Card, Card),
    community_cards: &[Card],
) -> Ordering {
    let eval1 = evaluate_hand(hand1, community_cards);
    let eval2 = evaluate_hand(hand2, community_cards);

    match eval1.rank.cmp(&eval2.rank) {
        Ordering::Equal => compare_kickers(&eval1.kickers, &eval2.kickers),
        other => other,
    }
}

/// 确定赢家
pub fn determine_winners(players: &[Player], community_cards: &[Card]) -> Vec<usize> {
    let active_players: Vec<_> = players
        .iter()
        .enumerate()
        .filter(|(_, p)| p.is_active && p.cards.is_some())
        .collect();

    if active_players.is_empty() {
        return vec![];
    }

    // 评估所有玩家手牌
    let mut evaluations: Vec<(usize, HandEvaluation)> = active_players
        .iter()
        .map(|(i, p)| {
            let cards = p.cards.unwrap();
            (*i, evaluate_hand(&cards, community_cards))
        })
        .collect();

    // 找到最佳牌型
    evaluations.sort_by(|(_, a), (_, b)| b.cmp(a));
    let best_evaluation = &evaluations[0].1;

    // 找出所有与最佳牌型相等的玩家
    evaluations
        .iter()
        .filter(|(_, eval)| {
            eval.rank == best_evaluation.rank
                && compare_kickers(&eval.kickers, &best_evaluation.kickers) == Ordering::Equal
        })
        .map(|(i, _)| *i)
        .collect()
}
