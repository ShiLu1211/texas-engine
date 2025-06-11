use rstest::rstest;
use texas_engine::*;

#[test]
fn test_deck_creation() {
    let deck = rules::create_shuffled_deck();
    assert_eq!(deck.len(), 52, "Deck should have 52 cards");

    let mut unique_cards = std::collections::HashSet::new();
    for card in deck {
        unique_cards.insert((card.suit, card.rank));
    }
    assert_eq!(unique_cards.len(), 52, "All cards should be unique");
}

#[test]
fn test_new_game_setup() {
    let players = vec![
        Player {
            id: "1".to_string(),
            name: "Alice".to_string(),
            chips: 1000,
            cards: None,
            is_active: true,
            current_bet: 0,
            has_acted: false,
        },
        Player {
            id: "2".to_string(),
            name: "Bob".to_string(),
            chips: 1000,
            cards: None,
            is_active: true,
            current_bet: 0,
            has_acted: false,
        },
    ];

    let game = state::TexasHoldem::new(players, 10, 20);

    assert_eq!(game.state.players.len(), 2);
    assert!(game.state.players[0].cards.is_some());
    assert!(game.state.players[1].cards.is_some());
    assert_eq!(game.state.pot, 30); // 小盲注10 + 大盲注20
    assert_eq!(game.state.stage, GameStage::PreFlop);
}

#[rstest]
#[case(PlayerAction::Fold, 30, 1000, false)] // Alice弃牌
#[case(PlayerAction::Call, 50, 980, true)] // Alice跟注20（补齐大盲）
fn test_preflop_actions(
    #[case] action: PlayerAction,
    #[case] expected_pot: u32,
    #[case] expected_alice_chips: u32,
    #[case] alice_active: bool,
) {
    let players = vec![
        Player {
            id: "1".to_string(),
            name: "Alice".to_string(),
            chips: 1000,
            cards: None,
            is_active: true,
            current_bet: 0,
            has_acted: false,
        },
        Player {
            id: "2".to_string(),
            name: "Bob".to_string(),
            chips: 1000,
            cards: None,
            is_active: true,
            current_bet: 20, // Bob是大盲注
            has_acted: false,
        },
    ];

    let mut game = state::TexasHoldem {
        state: GameState {
            players,
            community_cards: Vec::new(),
            pot: 30, // 小盲10 + 大盲20
            current_player_index: 0,
            stage: GameStage::PreFlop,
            dealer_position: 0,
            small_blind: 10,
            big_blind: 20,
        },
        deck: rules::create_shuffled_deck(),
    };

    // Alice行动
    game.handle_action(action).unwrap();

    assert_eq!(game.state.pot, expected_pot);
    assert_eq!(game.state.players[0].chips, expected_alice_chips);
    assert_eq!(game.state.players[0].is_active, alice_active);
}

#[test]
fn test_full_round() {
    let players = vec![
        Player {
            id: "1".to_string(),
            name: "Alice".to_string(),
            chips: 1000,
            cards: None,
            is_active: true,
            current_bet: 0,
            has_acted: false,
        },
        Player {
            id: "2".to_string(),
            name: "Bob".to_string(),
            chips: 1000,
            cards: None,
            is_active: true,
            current_bet: 0,
            has_acted: false,
        },
    ];

    // 使用构造器创建游戏
    let mut game = TexasHoldem::new(players, 10, 20);

    // Preflop
    game.handle_action(PlayerAction::Call).unwrap(); // Alice跟注20（补齐大盲）
    assert_eq!(game.state.players[0].chips, 980);
    assert_eq!(game.state.pot, 40);

    // Bob行动（大盲注后过牌）
    game.handle_action(PlayerAction::Check).unwrap();
    assert_eq!(game.state.stage, GameStage::Flop);
    assert_eq!(game.state.community_cards.len(), 3);

    // Flop
    game.handle_action(PlayerAction::Check).unwrap(); // Alice过牌
    game.handle_action(PlayerAction::Check).unwrap(); // Bob过牌
    assert_eq!(game.state.stage, GameStage::Turn);
    assert_eq!(game.state.community_cards.len(), 4);

    // Turn
    game.handle_action(PlayerAction::Bet(50)).unwrap(); // Alice下注50
    assert_eq!(game.state.players[0].chips, 930);
    assert_eq!(game.state.pot, 90);

    game.handle_action(PlayerAction::Call).unwrap(); // Bob跟注50
    assert_eq!(game.state.players[1].chips, 930);
    assert_eq!(game.state.pot, 140);
    assert_eq!(game.state.stage, GameStage::River);
    assert_eq!(game.state.community_cards.len(), 5);

    // River
    game.handle_action(PlayerAction::Check).unwrap(); // Alice过牌
    game.handle_action(PlayerAction::Check).unwrap(); // Bob过牌
    assert_eq!(game.state.stage, GameStage::Showdown);
}
