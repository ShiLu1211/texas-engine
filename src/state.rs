use super::rules::*;
use super::shared::*;

/// 德州扑克游戏状态机
pub struct TexasHoldem {
    pub state: GameState,
    pub deck: Vec<Card>,
}

/// Side pot 表示一个筹码池（主池或边池）
#[derive(Debug)]
struct SidePot {
    amount: u32,
    eligible_players: Vec<usize>, // 玩家索引
}

impl TexasHoldem {
    /// 创建新游戏
    pub fn new(players: Vec<Player>, small_blind: u32, big_blind: u32) -> Self {
        let mut game = TexasHoldem {
            state: GameState {
                players,
                community_cards: Vec::new(),
                pot: 0,
                current_player_index: 0,
                stage: GameStage::PreFlop,
                dealer_position: 0,
                small_blind,
                big_blind,
            },
            deck: create_shuffled_deck(),
        };

        game.setup_new_hand();
        game
    }

    /// 设置新的一局
    fn setup_new_hand(&mut self) {
        // 重置状态
        self.state.community_cards.clear();
        self.state.pot = 0;
        self.state.stage = GameStage::PreFlop;

        // 重置玩家状态
        for player in &mut self.state.players {
            player.cards = None;
            player.is_active = true;
            player.has_acted = false;
            player.current_bet = 0;
            player.total_bet_in_hand = 0;
        }

        // 发牌
        self.deal_cards();

        // 设置庄家位置和初始行动玩家
        self.state.current_player_index =
            (self.state.dealer_position + 3) % self.state.players.len();

        // 下盲注
        self.post_blinds();
    }

    /// 下盲注
    fn post_blinds(&mut self) {
        let small_blind_pos = (self.state.dealer_position + 1) % self.state.players.len();
        let big_blind_pos = (self.state.dealer_position + 2) % self.state.players.len();

        if let Some(player) = self.state.players.get_mut(small_blind_pos) {
            let amount = player.chips.min(self.state.small_blind);
            player.chips -= amount;
            player.current_bet = amount;
            player.total_bet_in_hand += amount;
            self.state.pot += amount;
        }

        if let Some(player) = self.state.players.get_mut(big_blind_pos) {
            let amount = player.chips.min(self.state.big_blind);
            player.chips -= amount;
            player.current_bet = amount;
            player.total_bet_in_hand += amount;
            self.state.pot += amount;
        }
    }

    /// 发牌给玩家
    fn deal_cards(&mut self) {
        for player in &mut self.state.players {
            if self.deck.len() >= 2 {
                let card1 = self.deck.pop().unwrap();
                let card2 = self.deck.pop().unwrap();
                player.cards = Some((card1, card2));
            }
        }
    }

    /// 处理玩家行动
    pub fn handle_action(&mut self, action: PlayerAction) -> Result<(), GameError> {
        // 提前计算当前轮次的下注额
        let current_bet_round = self.current_bet_round();
        let player_index = self.state.current_player_index;

        // 对于需要重置has_acted的动作，提前调用
        match &action {
            PlayerAction::Bet(_) | PlayerAction::Raise(_) => {
                self.reset_has_acted();
            }
            _ => {}
        }

        let player = self
            .state
            .players
            .get_mut(player_index)
            .ok_or(GameError::PlayerNotFound)?;

        if !player.is_active {
            return Err(GameError::InvalidAction);
        }

        match action {
            PlayerAction::Fold => {
                player.is_active = false;
                player.has_acted = true;
                self.advance_to_next_player();
            }
            PlayerAction::Check => {
                if current_bet_round > player.current_bet {
                    return Err(GameError::InvalidAction);
                }
                player.has_acted = true;
                self.advance_to_next_player();
            }
            PlayerAction::Bet(amount) => {
                if current_bet_round > 0 {
                    return Err(GameError::InvalidAction); // 只能加注，不能下注
                }
                if amount > player.chips {
                    return Err(GameError::InsufficientChips);
                }
                player.chips -= amount;
                player.current_bet += amount;
                player.total_bet_in_hand += amount;
                self.state.pot += amount;
                player.has_acted = true;
                self.advance_to_next_player();
            }
            PlayerAction::Raise(amount) => {
                if current_bet_round == 0 {
                    return Err(GameError::InvalidAction); // 没有下注时不能加注
                }

                let total_needed = current_bet_round + amount;
                if total_needed > player.chips + player.current_bet {
                    return Err(GameError::InsufficientChips);
                }

                let chips_to_put = total_needed - player.current_bet;
                player.chips -= chips_to_put;
                player.current_bet += chips_to_put;
                player.total_bet_in_hand += chips_to_put;
                self.state.pot += chips_to_put;
                player.has_acted = true;
                self.advance_to_next_player();
            }
            PlayerAction::Call => {
                let amount_to_call = current_bet_round - player.current_bet;
                if amount_to_call == 0 {
                    return Err(GameError::InvalidAction); // 无需跟注
                }

                if amount_to_call > player.chips {
                    return Err(GameError::InsufficientChips);
                }

                player.chips -= amount_to_call;
                player.current_bet += amount_to_call;
                player.total_bet_in_hand += amount_to_call;
                self.state.pot += amount_to_call;
                player.has_acted = true;
                self.advance_to_next_player();
            }
        }

        self.check_round_completion()?;
        Ok(())
    }

    /// 获取当前轮次的下注额（本轮最高下注额）
    fn current_bet_round(&self) -> u32 {
        self.state
            .players
            .iter()
            .map(|p| p.current_bet)
            .max()
            .unwrap_or(0)
    }

    /// 推进到下一位玩家
    fn advance_to_next_player(&mut self) {
        let mut next_index = (self.state.current_player_index + 1) % self.state.players.len();
        let start_index = next_index;

        loop {
            let player = &self.state.players[next_index];
            if player.is_active && player.chips > 0 {
                self.state.current_player_index = next_index;
                return;
            }

            next_index = (next_index + 1) % self.state.players.len();
            if next_index == start_index {
                // 所有玩家都已行动或弃牌
                self.state.current_player_index = next_index;
                return;
            }
        }
    }

    /// 检查当前阶段是否完成
    fn check_round_completion(&mut self) -> Result<(), GameError> {
        let active_players: Vec<_> = self.state.players.iter().filter(|p| p.is_active).collect();

        if active_players.len() <= 1 {
            self.state.stage = GameStage::Showdown;
            return Ok(());
        }

        let current_bet_round = self.current_bet_round();

        // 检查所有活跃玩家是否已完成本轮下注
        let all_acted = active_players.iter().all(|p| {
            // 玩家已行动或没有筹码
            p.has_acted || p.chips == 0
        });

        // 检查所有玩家是否跟注或全下
        let all_called = active_players
            .iter()
            .all(|p| p.current_bet == current_bet_round || p.chips == 0);

        if all_acted && all_called {
            self.advance_to_next_stage()?;
        }

        Ok(())
    }

    fn reset_has_acted(&mut self) {
        for player in &mut self.state.players {
            player.has_acted = false;
        }
    }

    /// 推进到下一阶段
    fn advance_to_next_stage(&mut self) -> Result<(), GameError> {
        // 重置玩家的当前轮下注额
        for player in &mut self.state.players {
            player.current_bet = 0;
            player.has_acted = false;
        }

        match self.state.stage {
            GameStage::PreFlop => {
                // 发三张公共牌（翻牌）
                if self.deck.len() < 3 {
                    return Err(GameError::StageError);
                }
                for _ in 0..3 {
                    self.state.community_cards.push(self.deck.pop().unwrap());
                }
                self.state.stage = GameStage::Flop;
            }
            GameStage::Flop => {
                // 发一张公共牌（转牌）
                if self.deck.is_empty() {
                    return Err(GameError::StageError);
                }
                self.state.community_cards.push(self.deck.pop().unwrap());
                self.state.stage = GameStage::Turn;
            }
            GameStage::Turn => {
                // 发一张公共牌（河牌）
                if self.deck.is_empty() {
                    return Err(GameError::StageError);
                }
                self.state.community_cards.push(self.deck.pop().unwrap());
                self.state.stage = GameStage::River;
            }
            GameStage::River => {
                self.state.stage = GameStage::Showdown;
            }
            GameStage::Showdown => {
                self.resolve_showdown();
                // 游戏结束，准备新一局
                self.setup_new_hand();
            }
        }

        // 设置行动玩家
        self.state.current_player_index =
            (self.state.dealer_position + 1) % self.state.players.len();
        self.advance_to_next_player(); // 找到第一个有效玩家

        Ok(())
    }

    /// 计算边池
    fn compute_side_pots(&self) -> Vec<SidePot> {
        // 收集所有玩家的总下注额
        let mut bets: Vec<_> = self
            .state
            .players
            .iter()
            .enumerate()
            .map(|(i, p)| (i, p.total_bet_in_hand))
            .collect();

        // 按总下注额排序
        bets.sort_by_key(|(_, bet)| *bet);

        let mut pots = Vec::new();
        let mut last_bet = 0;

        for &(_, bet) in &bets {
            if bet > last_bet {
                // 计算当前层的下注额增量
                let increment = bet - last_bet;

                // 当前层合格的玩家：所有下注额大于等于当前下注额的玩家
                let eligible_players: Vec<usize> = bets
                    .iter()
                    .filter(|(_, b)| *b >= bet)
                    .map(|(i, _)| *i)
                    .collect();

                // 当前层边池的金额
                let amount = increment * eligible_players.len() as u32;

                pots.push(SidePot {
                    amount,
                    eligible_players,
                });

                last_bet = bet;
            }
        }

        pots
    }

    /// 在 Showdown 阶段结算赢家，分配筹码
    pub fn resolve_showdown(&mut self) {
        let side_pots = self.compute_side_pots();
        let evaluations = self.evaluate_all_hands();
        let mut winnings = vec![0; self.state.players.len()];

        // 处理每个边池
        for pot in side_pots {
            let mut best_eval: Option<&HandEvaluation> = None;
            let mut winners = Vec::new();

            // 找出此边池的最佳手牌
            for &player_index in &pot.eligible_players {
                if let Some(eval) = &evaluations[player_index] {
                    match best_eval {
                        None => {
                            best_eval = Some(eval);
                            winners = vec![player_index];
                        }
                        Some(current_best) => match eval.rank.cmp(&current_best.rank) {
                            std::cmp::Ordering::Greater => {
                                best_eval = Some(eval);
                                winners = vec![player_index];
                            }
                            std::cmp::Ordering::Equal => {
                                if compare_kickers(&eval.kickers, &current_best.kickers)
                                    == std::cmp::Ordering::Equal
                                {
                                    winners.push(player_index);
                                } else if compare_kickers(&eval.kickers, &current_best.kickers)
                                    == std::cmp::Ordering::Greater
                                {
                                    best_eval = Some(eval);
                                    winners = vec![player_index];
                                }
                            }
                            _ => {}
                        },
                    }
                }
            }

            // 分配边池筹码
            if !winners.is_empty() {
                let share = pot.amount / winners.len() as u32;
                for &winner in &winners {
                    winnings[winner] += share;
                }
            }
        }

        // 应用筹码分配
        for (i, amount) in winnings.into_iter().enumerate() {
            self.state.players[i].chips += amount;
        }
    }

    fn evaluate_all_hands(&self) -> Vec<Option<HandEvaluation>> {
        self.state
            .players
            .iter()
            .map(|p| {
                if p.is_active && p.cards.is_some() {
                    Some(evaluate_hand(
                        &p.cards.unwrap(),
                        &self.state.community_cards,
                    ))
                } else {
                    None
                }
            })
            .collect()
    }
}
