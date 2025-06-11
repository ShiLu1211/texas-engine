use texas_engine::{rules::HandRank, *};

#[test]
fn test_royal_flush() {
    let player_cards = (
        Card {
            suit: Suit::Hearts,
            rank: Rank::Ace,
        },
        Card {
            suit: Suit::Hearts,
            rank: Rank::King,
        },
    );

    let community_cards = vec![
        Card {
            suit: Suit::Hearts,
            rank: Rank::Queen,
        },
        Card {
            suit: Suit::Hearts,
            rank: Rank::Jack,
        },
        Card {
            suit: Suit::Hearts,
            rank: Rank::Ten,
        },
        Card {
            suit: Suit::Diamonds,
            rank: Rank::Two,
        },
        Card {
            suit: Suit::Clubs,
            rank: Rank::Three,
        },
    ];

    let evaluation = rules::evaluate_hand(&player_cards, &community_cards);
    assert_eq!(evaluation.rank, HandRank::RoyalFlush);
}

#[test]
fn test_straight_flush() {
    let player_cards = (
        Card {
            suit: Suit::Spades,
            rank: Rank::Nine,
        },
        Card {
            suit: Suit::Spades,
            rank: Rank::Eight,
        },
    );

    let community_cards = vec![
        Card {
            suit: Suit::Spades,
            rank: Rank::Seven,
        },
        Card {
            suit: Suit::Spades,
            rank: Rank::Six,
        },
        Card {
            suit: Suit::Spades,
            rank: Rank::Five,
        },
        Card {
            suit: Suit::Diamonds,
            rank: Rank::King,
        },
        Card {
            suit: Suit::Clubs,
            rank: Rank::Queen,
        },
    ];

    let evaluation = rules::evaluate_hand(&player_cards, &community_cards);
    assert_eq!(evaluation.rank, HandRank::StraightFlush);
    assert_eq!(evaluation.kickers, vec![Rank::Nine]);
}

#[test]
fn test_four_of_a_kind() {
    let player_cards = (
        Card {
            suit: Suit::Hearts,
            rank: Rank::Ace,
        },
        Card {
            suit: Suit::Diamonds,
            rank: Rank::Ace,
        },
    );

    let community_cards = vec![
        Card {
            suit: Suit::Clubs,
            rank: Rank::Ace,
        },
        Card {
            suit: Suit::Spades,
            rank: Rank::Ace,
        },
        Card {
            suit: Suit::Hearts,
            rank: Rank::King,
        },
        Card {
            suit: Suit::Diamonds,
            rank: Rank::Two,
        },
        Card {
            suit: Suit::Clubs,
            rank: Rank::Three,
        },
    ];

    let evaluation = rules::evaluate_hand(&player_cards, &community_cards);
    assert_eq!(evaluation.rank, HandRank::FourOfAKind);
    assert_eq!(evaluation.kickers, vec![Rank::Ace, Rank::King]);
}

#[test]
fn test_full_house() {
    let player_cards = (
        Card {
            suit: Suit::Hearts,
            rank: Rank::King,
        },
        Card {
            suit: Suit::Diamonds,
            rank: Rank::King,
        },
    );

    let community_cards = vec![
        Card {
            suit: Suit::Clubs,
            rank: Rank::King,
        },
        Card {
            suit: Suit::Spades,
            rank: Rank::Queen,
        },
        Card {
            suit: Suit::Hearts,
            rank: Rank::Queen,
        },
        Card {
            suit: Suit::Diamonds,
            rank: Rank::Two,
        },
        Card {
            suit: Suit::Clubs,
            rank: Rank::Three,
        },
    ];

    let evaluation = rules::evaluate_hand(&player_cards, &community_cards);
    assert_eq!(evaluation.rank, HandRank::FullHouse);
    assert_eq!(evaluation.kickers, vec![Rank::King, Rank::Queen]);
}

#[test]
fn test_flush() {
    let player_cards = (
        Card {
            suit: Suit::Hearts,
            rank: Rank::Ace,
        },
        Card {
            suit: Suit::Hearts,
            rank: Rank::Ten,
        },
    );

    let community_cards = vec![
        Card {
            suit: Suit::Hearts,
            rank: Rank::King,
        },
        Card {
            suit: Suit::Hearts,
            rank: Rank::Queen,
        },
        Card {
            suit: Suit::Diamonds,
            rank: Rank::Jack,
        },
        Card {
            suit: Suit::Hearts,
            rank: Rank::Two,
        },
        Card {
            suit: Suit::Clubs,
            rank: Rank::Three,
        },
    ];

    let evaluation = rules::evaluate_hand(&player_cards, &community_cards);
    assert_eq!(evaluation.rank, HandRank::Flush);
    assert_eq!(
        evaluation.kickers,
        vec![Rank::Ace, Rank::King, Rank::Queen, Rank::Ten, Rank::Two]
    );
}

#[test]
fn test_straight() {
    let player_cards = (
        Card {
            suit: Suit::Hearts,
            rank: Rank::Ten,
        },
        Card {
            suit: Suit::Diamonds,
            rank: Rank::Nine,
        },
    );

    let community_cards = vec![
        Card {
            suit: Suit::Clubs,
            rank: Rank::Eight,
        },
        Card {
            suit: Suit::Spades,
            rank: Rank::Seven,
        },
        Card {
            suit: Suit::Hearts,
            rank: Rank::Six,
        },
        Card {
            suit: Suit::Diamonds,
            rank: Rank::Two,
        },
        Card {
            suit: Suit::Clubs,
            rank: Rank::Three,
        },
    ];

    let evaluation = rules::evaluate_hand(&player_cards, &community_cards);
    assert_eq!(evaluation.rank, HandRank::Straight);
    assert_eq!(evaluation.kickers, vec![Rank::Ten]);
}

#[test]
fn test_straight_ace_low() {
    let player_cards = (
        Card {
            suit: Suit::Hearts,
            rank: Rank::Ace,
        },
        Card {
            suit: Suit::Diamonds,
            rank: Rank::Two,
        },
    );

    let community_cards = vec![
        Card {
            suit: Suit::Clubs,
            rank: Rank::Three,
        },
        Card {
            suit: Suit::Spades,
            rank: Rank::Four,
        },
        Card {
            suit: Suit::Hearts,
            rank: Rank::Five,
        },
        Card {
            suit: Suit::Diamonds,
            rank: Rank::King,
        },
        Card {
            suit: Suit::Clubs,
            rank: Rank::Queen,
        },
    ];

    let evaluation = rules::evaluate_hand(&player_cards, &community_cards);
    assert_eq!(evaluation.rank, HandRank::Straight);
    assert_eq!(evaluation.kickers, vec![Rank::Five]); // A-5顺子，最大牌是5
}

#[test]
fn test_three_of_a_kind() {
    let player_cards = (
        Card {
            suit: Suit::Hearts,
            rank: Rank::Jack,
        },
        Card {
            suit: Suit::Diamonds,
            rank: Rank::Jack,
        },
    );

    let community_cards = vec![
        Card {
            suit: Suit::Clubs,
            rank: Rank::Jack,
        },
        Card {
            suit: Suit::Spades,
            rank: Rank::Ten,
        },
        Card {
            suit: Suit::Hearts,
            rank: Rank::Nine,
        },
        Card {
            suit: Suit::Diamonds,
            rank: Rank::Two,
        },
        Card {
            suit: Suit::Clubs,
            rank: Rank::Three,
        },
    ];

    let evaluation = rules::evaluate_hand(&player_cards, &community_cards);
    assert_eq!(evaluation.rank, HandRank::ThreeOfAKind);
    assert_eq!(evaluation.kickers, vec![Rank::Jack, Rank::Ten, Rank::Nine]);
}

#[test]
fn test_two_pair() {
    let player_cards = (
        Card {
            suit: Suit::Hearts,
            rank: Rank::Queen,
        },
        Card {
            suit: Suit::Diamonds,
            rank: Rank::Queen,
        },
    );

    let community_cards = vec![
        Card {
            suit: Suit::Clubs,
            rank: Rank::Ten,
        },
        Card {
            suit: Suit::Spades,
            rank: Rank::Ten,
        },
        Card {
            suit: Suit::Hearts,
            rank: Rank::Nine,
        },
        Card {
            suit: Suit::Diamonds,
            rank: Rank::Two,
        },
        Card {
            suit: Suit::Clubs,
            rank: Rank::Three,
        },
    ];

    let evaluation = rules::evaluate_hand(&player_cards, &community_cards);
    assert_eq!(evaluation.rank, HandRank::TwoPair);
    assert_eq!(evaluation.kickers, vec![Rank::Queen, Rank::Ten, Rank::Nine]);
}

#[test]
fn test_one_pair() {
    let player_cards = (
        Card {
            suit: Suit::Hearts,
            rank: Rank::Ace,
        },
        Card {
            suit: Suit::Diamonds,
            rank: Rank::Ace,
        },
    );

    let community_cards = vec![
        Card {
            suit: Suit::Clubs,
            rank: Rank::King,
        },
        Card {
            suit: Suit::Spades,
            rank: Rank::Queen,
        },
        Card {
            suit: Suit::Hearts,
            rank: Rank::Jack,
        },
        Card {
            suit: Suit::Diamonds,
            rank: Rank::Two,
        },
        Card {
            suit: Suit::Clubs,
            rank: Rank::Three,
        },
    ];

    let evaluation = rules::evaluate_hand(&player_cards, &community_cards);
    assert_eq!(evaluation.rank, HandRank::OnePair);
    assert_eq!(
        evaluation.kickers,
        vec![Rank::Ace, Rank::King, Rank::Queen, Rank::Jack]
    );
}

#[test]
fn test_high_card() {
    let player_cards = (
        Card {
            suit: Suit::Hearts,
            rank: Rank::Ace,
        },
        Card {
            suit: Suit::Diamonds,
            rank: Rank::King,
        },
    );

    let community_cards = vec![
        Card {
            suit: Suit::Clubs,
            rank: Rank::Queen,
        },
        Card {
            suit: Suit::Spades,
            rank: Rank::Jack,
        },
        Card {
            suit: Suit::Hearts,
            rank: Rank::Nine,
        },
        Card {
            suit: Suit::Diamonds,
            rank: Rank::Two,
        },
        Card {
            suit: Suit::Clubs,
            rank: Rank::Three,
        },
    ];

    let evaluation = rules::evaluate_hand(&player_cards, &community_cards);
    assert_eq!(evaluation.rank, HandRank::HighCard);
    assert_eq!(
        evaluation.kickers,
        vec![Rank::Ace, Rank::King, Rank::Queen, Rank::Jack, Rank::Nine]
    );
}
