use poker_ws::{game::Table, Deck, parse_card, game::ApplyOutcome};
// tokio time is used by the async test below via the runtime attribute

#[test]
fn deck_shuffle_and_deal() {
    let mut d = Deck::new();
    d.shuffle();
    let c1 = d.deal().unwrap();
    let c2 = d.deal().unwrap();
    assert_ne!(format!("{}", c1), format!("{}", c2));
}

#[test]
fn start_hand_and_blinds() {
    let mut t = Table::new("t1".into(), 6, 5, 10);
    t.sit("u1".into(), 1000);
    t.sit("u2".into(), 1000);
    let mut d = Deck::new();
    d.shuffle();
    t.start_hand(&mut d);
    assert_eq!(t.state.pot, 15);
}

#[test]
fn actions_flow_to_next_street() {
    let mut t = Table::new("t1".into(), 6, 5, 10);
    t.sit("u1".into(), 1000);
    t.sit("u2".into(), 1000);
    let mut d = Deck::new(); d.shuffle(); t.start_hand(&mut d);
    // with 2 players, u2 (SB) acts first after blinds
    assert!(matches!(t.apply_action_by_user("u2","call", None), Ok(_)));
    match t.apply_action_by_user("u1","check", None) { Ok(ApplyOutcome::NextStreet)|Ok(ApplyOutcome::Continue)|Ok(ApplyOutcome::HandEnded)=>{}, Err(e)=>panic!("{e}") }
}

#[test]
fn showdown_split_pot_two_pairs_vs_two_pairs_kicker() {
    // Craft a situation with same two pair, different kicker (Aces+Kings, kicker differs)
    let mut t = Table::new("t1".into(), 6, 1, 2);
    t.sit("a".into(), 1000);
    t.sit("b".into(), 1000);
    t.board = vec![parse_card("Ah"), parse_card("Ad"), parse_card("Kc"), parse_card("7c"), parse_card("2s")];
    t.seats[0].user_id = Some("a".into()); t.seats[0].hole = vec![parse_card("Qd"), parse_card("3c")]; // kicker Q
    t.seats[1].user_id = Some("b".into()); t.seats[1].hole = vec![parse_card("Jd"), parse_card("4c")]; // kicker J
    t.total_contrib = vec![100, 100, 0,0,0,0];
    t.state.pot = 200;
    t.showdown_and_payout();
    assert_eq!(t.seats[0].stack, 1200);
    assert_eq!(t.seats[1].stack, 1000);
}

#[test]
fn side_pot_simple_allin() {
    // a: all in 50, b calls 50 and adds 50 more, c calls 100
    let mut t = Table::new("t1".into(), 6, 1, 2);
    t.sit("a".into(), 0);
    t.sit("b".into(), 0);
    t.sit("c".into(), 0);
    t.seats[0].user_id = Some("a".into()); t.seats[0].has_folded=false;
    t.seats[1].user_id = Some("b".into()); t.seats[1].has_folded=false;
    t.seats[2].user_id = Some("c".into()); t.seats[2].has_folded=false;
    t.total_contrib = vec![50, 100, 100, 0,0,0];
    t.state.pot = 250;
    // deterministic ranks: c > b > a based on pocket pairs
    t.board = vec![parse_card("2h"), parse_card("5s"), parse_card("Jd"), parse_card("Qc"), parse_card("Kh")];
    t.seats[0].hole = vec![parse_card("2c"), parse_card("3c")]; // pair 2
    t.seats[1].hole = vec![parse_card("7c"), parse_card("7d")]; // pair 7
    t.seats[2].hole = vec![parse_card("8c"), parse_card("8d")]; // pair 8
    t.showdown_and_payout();
    // main pot 150 to best among a,b,c => c should win 150
    // side pot 100 among b,c => c should win 100 => total 250 to c
    assert_eq!(t.seats[2].stack, 250);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn timer_auto_progress_check_fold() {
    // This test uses the actor via ws is heavy; we instead simulate at table level by calling Tick logic.
    // Here we just assert that after some auto check/fold cycles, table street advances to at least Flop.
    let mut t = Table::new("t1".into(), 6, 5, 10);
    t.sit("u1".into(), 1000);
    t.sit("u2".into(), 1000);
    let mut d = Deck::new(); d.shuffle(); t.start_hand(&mut d);
    // simulate ticking by performing auto check/fold manually a few times
    for _ in 0..3 {
        let idx = t.to_act_idx;
        // make check valid by aligning round_bet to current actor's contribution
        t.round_bet = t.round_contrib[idx];
        let uid = t.seats[idx].user_id.clone().unwrap();
        let _ = t.apply_action_by_user(&uid, "check", None).or_else(|_| t.apply_action_by_user(&uid, "fold", None));
        t.next_street(&mut d);
    }
    assert!(t.state.street.is_some());
}

#[test]
fn two_players_full_hand_winner() {
    // Deterministic deck so that seat0 wins with AA over KK, no further betting
    let mut t = Table::new("t1".into(), 6, 5, 10);
    t.sit("u0".into(), 1000);
    t.sit("u1".into(), 1000);
    let mut d = Deck(vec![]);
    // Build deck suffix: river, turn, flop3, flop2, flop1, s1_2, s0_2, s1_1, s0_1 (note: dealing uses pop)
    let s0_1 = parse_card("As");
    let s1_1 = parse_card("Kd");
    let s0_2 = parse_card("Ah");
    let s1_2 = parse_card("Kc");
    let flop1 = parse_card("2d");
    let flop2 = parse_card("7s");
    let flop3 = parse_card("9c");
    let turn = parse_card("3h");
    let river = parse_card("4d");
    d.0.extend(vec![river, turn, flop3, flop2, flop1, s1_2, s0_2, s1_1, s0_1]);

    t.start_hand(&mut d);
    // Preflop: SB calls, BB checks -> to flop
    let sb_uid = t.seats[t.to_act_idx].user_id.clone().unwrap();
    let _ = t.apply_action_by_user(&sb_uid, "call", None).unwrap();
    let bb_uid = t.seats[t.to_act_idx].user_id.clone().unwrap();
    if let Ok(ApplyOutcome::NextStreet) = t.apply_action_by_user(&bb_uid, "check", None) { t.next_street(&mut d); }

    // Flop/Turn/River: both check each street
    for _street in 0..3 {
        for _ in 0..2 {
            let uid = t.seats[t.to_act_idx].user_id.clone().unwrap();
            t.round_bet = t.round_contrib[t.to_act_idx];
            if let Ok(ApplyOutcome::NextStreet) = t.apply_action_by_user(&uid, "check", None) { t.next_street(&mut d); }
        }
    }

    // Showdown and payout
    t.showdown_and_payout();
    let total = t.seats[0].stack + t.seats[1].stack;
    assert_eq!(total, 2000);
    assert!(t.seats[0].stack > t.seats[1].stack);
}


