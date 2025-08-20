use crate::{Deck, Card, TableState, Street, Rank, Suit};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyOutcome { Continue, NextStreet, HandEnded }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Seat {
    pub user_id: Option<String>,
    pub stack: u64,
    pub hole: Vec<Card>,
    pub sitting_out: bool,
    pub has_folded: bool,
    pub is_allin: bool,
    #[serde(skip)]
    pub acted_in_round: bool,
}

impl Seat {
    pub fn empty() -> Self { Self { user_id: None, stack: 0, hole: vec![], sitting_out: false, has_folded: false, is_allin: false, acted_in_round: false } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub id: String,
    pub max_seats: usize,
    pub seats: Vec<Seat>,
    pub dealer_idx: usize,
    pub to_act_idx: usize,
    pub board: Vec<Card>,
    pub state: TableState,
    pub round_bet: u64,
    pub round_contrib: Vec<u64>,
    pub total_contrib: Vec<u64>,
}

impl Table {
    pub fn new(id: String, max_seats: usize, sb: u64, bb: u64) -> Self {
        Self { id, max_seats, seats: (0..max_seats).map(|_| Seat::empty()).collect(), dealer_idx: 0, to_act_idx: 0, board: vec![], state: TableState { small_blind: sb, big_blind: bb, pot: 0, street: None }, round_bet: 0, round_contrib: vec![0; max_seats], total_contrib: vec![0; max_seats] }
    }

    pub fn sit(&mut self, user_id: String, stack: u64) -> bool {
        if let Some(slot) = self.seats.iter_mut().find(|s| s.user_id.is_none()) {
            *slot = Seat { user_id: Some(user_id), stack, hole: vec![], sitting_out: false, has_folded: false, is_allin: false, acted_in_round: false };
            true
        } else { false }
    }

    pub fn active_player_count(&self) -> usize {
        self.seats.iter().filter(|s| s.user_id.is_some() && !s.sitting_out).count()
    }

    fn next_occupied_from(&self, mut idx: usize) -> usize {
        for _ in 0..self.max_seats {
            idx = (idx + 1) % self.max_seats;
            let s = &self.seats[idx];
            if s.user_id.is_some() && !s.sitting_out && !s.is_allin && !s.has_folded { return idx; }
        }
        idx
    }

    pub fn start_hand(&mut self, deck: &mut Deck) {
        self.state.start_hand();
        self.board.clear();
        self.round_bet = 0;
        self.round_contrib.fill(0);
        self.total_contrib.fill(0);
        for seat in &mut self.seats { seat.hole.clear(); seat.has_folded = false; seat.is_allin = false; seat.acted_in_round = false; }
        // deal 2 cards each
        for _ in 0..2 { for seat in &mut self.seats { if seat.user_id.is_some() && !seat.sitting_out { if let Some(c)=deck.deal() { seat.hole.push(c); } } } }
        self.state.street = Some(Street::Preflop);
        // blinds
        let sb_idx = self.next_occupied_from(self.dealer_idx);
        let bb_idx = self.next_occupied_from(sb_idx);
        let sb_amt = self.state.small_blind.min(self.seats[sb_idx].stack);
        self.seats[sb_idx].stack -= sb_amt; self.round_contrib[sb_idx] += sb_amt; self.total_contrib[sb_idx] += sb_amt; self.state.pot += sb_amt;
        let bb_amt = self.state.big_blind.min(self.seats[bb_idx].stack);
        self.seats[bb_idx].stack -= bb_amt; self.round_contrib[bb_idx] += bb_amt; self.total_contrib[bb_idx] += bb_amt; self.state.pot += bb_amt;
        self.round_bet = self.round_contrib[bb_idx];
        self.to_act_idx = self.next_occupied_from(bb_idx);
    }

    pub fn next_street(&mut self, deck: &mut Deck) {
        match self.state.street {
            Some(Street::Preflop) => { self.board.extend(deck.deal_n(3)); self.state.street = Some(Street::Flop); }
            Some(Street::Flop) => { self.board.extend(deck.deal_n(1)); self.state.street = Some(Street::Turn); }
            Some(Street::Turn) => { self.board.extend(deck.deal_n(1)); self.state.street = Some(Street::River); }
            Some(Street::River) => { self.state.street = Some(Street::Showdown); }
            _ => {}
        }
        // reset round state
        self.round_bet = 0;
        self.round_contrib.fill(0);
        for s in &mut self.seats { s.acted_in_round = false; }
        // on postflop, first to act is next from dealer
        if let Some(street) = self.state.street {
            if street != Street::Showdown {
                self.to_act_idx = self.next_occupied_from(self.dealer_idx);
            }
        }
    }

    fn alive_players(&self) -> Vec<usize> {
        (0..self.max_seats).filter(|&i| {
            let s = &self.seats[i];
            s.user_id.is_some() && !s.sitting_out && !s.has_folded && (s.stack > 0 || self.round_contrib[i] > 0 || self.total_contrib[i] > 0)
        }).collect()
    }

    fn all_matched(&self) -> bool {
        for i in 0..self.max_seats {
            let s = &self.seats[i];
            if s.user_id.is_none() || s.sitting_out || s.has_folded || s.is_allin { continue; }
            if self.round_contrib[i] != self.round_bet { return false; }
            if !s.acted_in_round { return false; }
        }
        true
    }

    pub fn apply_action_by_user(&mut self, user_id: &str, action: &str, amount: Option<u64>) -> Result<ApplyOutcome, String> {
        let seat_idx = self.seats.iter().position(|s| s.user_id.as_deref() == Some(user_id)).ok_or("not seated")?;
        if self.to_act_idx != seat_idx { return Err("not your turn".into()); }
        if self.state.street == Some(Street::Showdown) || self.state.street.is_none() { return Err("hand not active".into()); }
        let to_call = self.round_bet.saturating_sub(self.round_contrib[seat_idx]);
        match action {
            "fold" => { self.seats[seat_idx].has_folded = true; self.seats[seat_idx].acted_in_round = true; }
            "check" => { if to_call != 0 { return Err("cannot check".into()); } self.seats[seat_idx].acted_in_round = true; }
            "call" => {
                let s = &mut self.seats[seat_idx];
                let pay = to_call.min(s.stack);
                s.stack -= pay; self.round_contrib[seat_idx] += pay; self.total_contrib[seat_idx] += pay; self.state.pot += pay; s.acted_in_round = true; if s.stack == 0 { s.is_allin = true; }
            }
            "raise" => {
                let raise_by = amount.unwrap_or(0);
                if raise_by < self.state.big_blind { return Err("min raise".into()); }
                let s = &mut self.seats[seat_idx];
                let need = to_call + raise_by;
                if need == 0 { return Err("bad raise".into()); }
                let pay = need.min(s.stack);
                s.stack -= pay; self.round_contrib[seat_idx] += pay; self.total_contrib[seat_idx] += pay; self.state.pot += pay; s.acted_in_round = true; if s.stack == 0 { s.is_allin = true; }
                self.round_bet = self.round_contrib[seat_idx];
                // on raise, others need to act again
                for i in 0..self.max_seats { if i != seat_idx { self.seats[i].acted_in_round = false; } }
            }
            _ => return Err("unknown action".into()),
        }

        // check if only one player remains
        let alive: Vec<_> = self.alive_players();
        if alive.len() <= 1 {
            if let Some(&winner) = alive.first() { self.seats[winner].stack += self.state.pot; }
            self.state.pot = 0; self.state.street = None; self.board.clear(); self.round_contrib.fill(0); self.total_contrib.fill(0);
            self.dealer_idx = self.next_occupied_from(self.dealer_idx);
            return Ok(ApplyOutcome::HandEnded);
        }

        // advance street if matched
        if self.all_matched() {
            return Ok(ApplyOutcome::NextStreet);
        } else {
            self.to_act_idx = self.next_occupied_from(seat_idx);
            return Ok(ApplyOutcome::Continue);
        }
    }

    pub fn showdown_and_payout(&mut self) {
        // Build side pots from total_contrib
        let mut remaining: Vec<u64> = self.total_contrib.clone();
        let mut pots: Vec<(u64, Vec<usize>)> = Vec::new();
        loop {
            let mut min_pos: Option<u64> = None;
            for (i, &c) in remaining.iter().enumerate() {
                if c > 0 { min_pos = Some(match min_pos { Some(m) => m.min(c), None => c }); }
            }
            let Some(layer) = min_pos else { break };
            let mut amount = 0u64;
            let mut eligible: Vec<usize> = Vec::new();
            for i in 0..remaining.len() {
                if remaining[i] > 0 { amount += layer; remaining[i] -= layer; }
                // eligible if player has not folded and had chips in this layer
                if self.seats[i].user_id.is_some() && !self.seats[i].has_folded && !self.seats[i].sitting_out && self.total_contrib[i] > 0 {
                    eligible.push(i);
                }
            }
            pots.push((amount, eligible));
        }

        // Evaluate all active players
        let mut ranks: Vec<Option<HandRank>> = vec![None; self.max_seats];
        for i in 0..self.max_seats { if self.seats[i].user_id.is_some() && !self.seats[i].has_folded {
            ranks[i] = Some(best_rank(&self.seats[i].hole, &self.board));
        }}

        for (mut amount, eligible) in pots.into_iter() {
            if amount == 0 || eligible.is_empty() { continue; }
            // pick winners
            let mut best: Option<HandRank> = None;
            let mut winners: Vec<usize> = Vec::new();
            for &i in &eligible { if let Some(r) = &ranks[i] {
                if best.is_none() || r > best.as_ref().unwrap() { best = Some(*r); winners.clear(); winners.push(i); }
                else if Some(r) == best.as_ref() { winners.push(i); }
            }}
            if winners.is_empty() { continue; }
            let share = amount / winners.len() as u64;
            let mut remainder = amount - share * winners.len() as u64;
            for &w in &winners { self.seats[w].stack += share; }
            // assign remainder to lowest index winners for determinism
            winners.iter().take(remainder as usize).for_each(|&w| { self.seats[w].stack += 1; });
        }

        self.state.pot = 0; self.state.street = None; self.board.clear(); self.round_contrib.fill(0); self.total_contrib.fill(0);
        self.dealer_idx = self.next_occupied_from(self.dealer_idx);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HandRank(pub u8, pub [u8;5]);

fn best_rank(hole: &Vec<Card>, board: &Vec<Card>) -> HandRank {
    let mut cards = hole.clone(); cards.extend(board.iter().copied());
    // choose best 5 out of up to 7
    let n = cards.len();
    let mut best = HandRank(0, [0;5]);
    let idxs: Vec<usize> = (0..n).collect();
    let combs = combinations(&idxs, 5);
    for c in combs {
        let five = [cards[c[0]], cards[c[1]], cards[c[2]], cards[c[3]], cards[c[4]]];
        let r = eval5(&five);
        if r > best { best = r; }
    }
    best
}

fn combinations(items: &Vec<usize>, k: usize) -> Vec<Vec<usize>> {
    let mut res = Vec::new();
    let n = items.len();
    let mut idx: Vec<usize> = (0..k).collect();
    loop {
        res.push(idx.iter().map(|&i| items[i]).collect());
        let mut i = k; while i>0 { i-=1; if idx[i] != i + n - k { idx[i]+=1; for j in i+1..k { idx[j]=idx[j-1]+1; } break; } }
        if idx[0] == n - k { break; }
    }
    res
}

fn rank_value(rank: Rank) -> u8 { rank as u8 }

fn eval5(cards: &[Card;5]) -> HandRank {
    // sort by rank desc
    let mut ranks: Vec<u8> = cards.iter().map(|c| rank_value(c.0)).collect();
    ranks.sort_unstable_by(|a,b| b.cmp(a));
    // count occurrences
    let mut counts = [0u8;15]; // 2..14
    for &r in &ranks { counts[r as usize]+=1; }
    let is_flush = cards.iter().all(|c| c.1 == cards[0].1);
    let is_straight = is_straight_sorted(&mut ranks.clone());

    if is_flush && is_straight { return HandRank(8, normalize_kickers(&ranks)); }

    // four of a kind
    if let Some((quad, kicker)) = find_n_of_a_kind(&counts, 4, &ranks) { return HandRank(7, [quad, quad, quad, quad, kicker]); }
    // full house
    if let Some(trip) = find_value_with_count(&counts, 3) { if let Some(pair) = find_second_pair(&counts, trip) { return HandRank(6, [trip, trip, trip, pair, pair]); } }
    if is_flush { return HandRank(5, normalize_kickers(&ranks)); }
    if is_straight { return HandRank(4, normalize_kickers(&ranks)); }
    if let Some((trip, kickers)) = find_three(&counts, &ranks) { return HandRank(3, [trip, trip, trip, kickers[0], kickers[1]]); }
    if let Some((hp, lp, kicker)) = find_two_pair(&counts, &ranks) { return HandRank(2, [hp, hp, lp, lp, kicker]); }
    if let Some((pair, ks)) = find_pair(&counts, &ranks) { return HandRank(1, [pair, pair, ks[0], ks[1], ks[2]]); }
    HandRank(0, normalize_kickers(&ranks))
}

fn is_straight_sorted(ranks: &mut Vec<u8>) -> bool {
    // handle wheel A-2-3-4-5
    let mut uniq = ranks.clone(); uniq.dedup();
    if uniq.len() < 5 { return false; }
    // try sequences including wheel
    let mut seq = 1; for i in 0..uniq.len()-1 { if uniq[i] == uniq[i+1]+1 { seq+=1; if seq>=5 { return true; } } else { seq=1; } }
    // wheel
    uniq.sort_unstable(); if uniq.ends_with(&[5,4,3,2]) && ranks.contains(&14) { return true; }
    false
}

fn normalize_kickers(r: &Vec<u8>) -> [u8;5] { [r[0], r[1], r[2], r[3], r[4]] }

fn find_value_with_count(counts: &[u8;15], n: u8) -> Option<u8> {
    for v in (2..=14).rev() { if counts[v as usize] == n { return Some(v as u8); } }
    None
}
fn find_second_pair(counts: &[u8;15], exclude: u8) -> Option<u8> {
    for v in (2..=14).rev() { if v as u8 != exclude && counts[v as usize] >= 2 { return Some(v as u8); } }
    None
}
fn find_n_of_a_kind(counts: &[u8;15], n: u8, ranks: &Vec<u8>) -> Option<(u8,u8)> {
    if let Some(v) = find_value_with_count(counts, n) { let kicker = ranks.iter().copied().find(|&x| x != v).unwrap_or(2); return Some((v, kicker)); } None
}
fn find_three(counts: &[u8;15], ranks: &Vec<u8>) -> Option<(u8,[u8;2])> {
    if let Some(v) = find_value_with_count(counts, 3) {
        let mut ks: Vec<u8> = ranks.iter().copied().filter(|&x| x != v).collect(); ks.sort_unstable_by(|a,b| b.cmp(a)); ks.truncate(2);
        return Some((v, [ks[0], ks[1]]));
    } None
}
fn find_pair(counts: &[u8;15], ranks: &Vec<u8>) -> Option<(u8,[u8;3])> {
    if let Some(v) = find_value_with_count(counts, 2) {
        let mut ks: Vec<u8> = ranks.iter().copied().filter(|&x| x != v).collect(); ks.sort_unstable_by(|a,b| b.cmp(a)); ks.truncate(3);
        return Some((v, [ks[0], ks[1], ks[2]]));
    } None
}
fn find_two_pair(counts: &[u8;15], ranks: &Vec<u8>) -> Option<(u8,u8,u8)> {
    let mut pairs: Vec<u8> = Vec::new();
    for v in (2..=14).rev() { if counts[v as usize] >= 2 { pairs.push(v as u8); if pairs.len()==2 { break; } } }
    if pairs.len()==2 {
        let kicker = ranks.iter().copied().find(|&x| x != pairs[0] && x != pairs[1]).unwrap_or(2);
        return Some((pairs[0], pairs[1], kicker));
    }
    None
}



