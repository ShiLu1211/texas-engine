use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use axum::{
    extract::{State, ws::{WebSocketUpgrade, WebSocket, Message}},
    response::IntoResponse,
    routing::get,
    Router,
};
use tower_http::services::ServeDir;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex};
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;
use axum::extract::ws::Message as WsMessage;
use futures::{StreamExt, SinkExt};
use tokio::time::{sleep, Duration};

// Reuse simple poker types scaffold
use poker_ws::{Deck, TableState};
use poker_ws::game::{Table, ApplyOutcome};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")] 
enum ClientAction {
    Join { table_id: String, buy_in: u64, client_msg_id: String },
    Action { table_id: String, hand_id: String, action: String, amount: Option<u64>, client_msg_id: String },
    CreateRoom { table_id: Option<String>, config: RoomConfig, client_msg_id: String },
    JoinRoom { table_id: String, client_msg_id: String },
    Rebuy { table_id: String, client_msg_id: String },
    Ready { table_id: String, client_msg_id: String, ready: bool },
    LeaveRoom { table_id: String, client_msg_id: String },
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")] 
enum ServerEvent {
    Welcome { msg: String },
    PlayerJoined { table_id: String },
    ActionAck { table_id: String, hand_id: String, action: String },
    TableSnapshot { table: poker_ws::game::Table, ready: HashMap<String, bool>, to_act_uid: Option<String>, ms_left: Option<u64> },
    RoomCreated { table_id: String },
    RoomClosed { table_id: String },
    PlayerReady { table_id: String, client_msg_id: String, ready: bool },
    PlayerLeft { table_id: String, client_msg_id: String },
    GameStartCountdown { table_id: String, ms_left: u64 },
    Error { message: String },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct RoomConfig {
    small_blind: u64,
    big_blind: u64,
    starting_stack: u64,
    rebuy_hands: u32,
    room_duration_sec: u64,
    action_time_ms: u64,
}

impl Default for RoomConfig {
    fn default() -> Self {
        Self {
            small_blind: 5,
            big_blind: 10,
            starting_stack: 1000,
            rebuy_hands: 0,
            room_duration_sec: 0,
            action_time_ms: 8000,
        }
    }
}

type ClientTx = mpsc::UnboundedSender<WsMessage>;

enum ActorMsg {
    Client(ClientAction),
    Subscribe(ClientTx),
    Tick,
}

struct TableActor {
    table_id: String,
    rx: mpsc::UnboundedReceiver<ActorMsg>,
    subscribers: Vec<ClientTx>,
    state: TableState,
    deck: Deck,
    table: Table,
    host_user: String,
    config: RoomConfig,
    rebuys_left: HashMap<String, u32>,
    room_end_at: Option<Instant>,
    action_deadline: Option<Instant>,
    ready_status: HashMap<String, bool>,
    countdown_end: Option<Instant>,
}

impl TableActor {
    fn spawn(table_id: String) -> mpsc::UnboundedSender<ActorMsg> {
        Self::spawn_with_config(table_id, "host".into(), RoomConfig::default())
    }

    fn spawn_with_config(table_id: String, host_user: String, config: RoomConfig) -> mpsc::UnboundedSender<ActorMsg> {
        let (tx, rx) = mpsc::unbounded_channel();
        let tx_return = tx.clone();
        let mut actor = TableActor {
            table_id: table_id.clone(),
            rx,
            subscribers: Vec::new(),
            state: TableState::default(),
            deck: Deck::new(),
            table: Table::new(table_id.clone(), 6, config.small_blind, config.big_blind),
            host_user,
            config: config.clone(),
            rebuys_left: HashMap::new(),
            room_end_at: if config.room_duration_sec > 0 { Some(Instant::now() + Duration::from_secs(config.room_duration_sec)) } else { None },
            action_deadline: None,
            ready_status: HashMap::new(),
            countdown_end: None,
        };
        tokio::spawn(async move {
            actor.deck.shuffle();
            // kick off a periodic tick for timers
            let tick_tx = tx.clone();
            tokio::spawn(async move {
                loop { sleep(Duration::from_millis(200)).await; let _ = tick_tx.send(ActorMsg::Tick); }
            });
            while let Some(msg) = actor.rx.recv().await {
                match msg {
                    ActorMsg::Subscribe(client_tx) => {
                        actor.subscribers.push(client_tx);
                        let _ = actor.broadcast(&ServerEvent::Welcome { msg: format!("joined {}", actor.table_id) });
                    }
                    ActorMsg::Client(ClientAction::CreateRoom { .. }) => {
                        // Room creation is handled in the router before actor spawn
                    }
                    ActorMsg::Tick => {
                        // simplistic timer: if a hand is active and no action for a while, auto action
                        // In a real impl, track per-hand/per-seat deadlines. Here we just auto-progress by check/fold on tick when waiting for action.
                        if actor.table.state.street.is_some() && actor.table.state.street != Some(poker_ws::Street::Showdown) {
                            if let Some(end) = actor.room_end_at { if Instant::now() >= end { let _ = actor.broadcast(&ServerEvent::RoomClosed { table_id: actor.table_id.clone() }); continue; } }
                            if let Some(dl) = actor.action_deadline { if Instant::now() < dl { continue; } }
                            let to_act = actor.table.to_act_idx;
                            if let Some(uid) = actor.table.seats[to_act].user_id.clone() {
                                let _ = actor.table.apply_action_by_user(&uid, "check", None)
                                    .or_else(|_| actor.table.apply_action_by_user(&uid, "fold", None));
                                actor.table.next_street(&mut actor.deck);
                                actor.action_deadline = Some(Instant::now() + Duration::from_millis(actor.config.action_time_ms));
                                if actor.table.state.street == Some(poker_ws::Street::Showdown) { actor.table.showdown_and_payout(); }
                                let to_act_uid = actor.table.seats.get(actor.table.to_act_idx).and_then(|s| s.user_id.clone());
                                let ms_left = actor.action_deadline.map(|dl| dl.saturating_duration_since(Instant::now()).as_millis() as u64);
                                let _ = actor.broadcast(&ServerEvent::TableSnapshot { table: actor.table.clone(), ready: actor.ready_status.clone(), to_act_uid, ms_left });
                            }
                        } else {
                            // if hand not active, handle start countdown
                            if actor.table.active_player_count() >= 2 && actor.table.state.street.is_none() {
                                // check all ready
                                let mut all_ready = true;
                                for seat in &actor.table.seats {
                                    if let Some(uid) = &seat.user_id {
                                        if !seat.sitting_out {
                                            if *actor.ready_status.get(uid).unwrap_or(&false) == false { all_ready = false; break; }
                                        }
                                    }
                                }
                                if all_ready && actor.room_end_at.map_or(true, |end| Instant::now() < end) {
                                    if actor.countdown_end.is_none() { actor.countdown_end = Some(Instant::now() + Duration::from_millis(2000)); }
                                    if let Some(end) = actor.countdown_end {
                                        if Instant::now() >= end {
                                            actor.table.start_hand(&mut actor.deck);
                                            actor.action_deadline = Some(Instant::now() + Duration::from_millis(actor.config.action_time_ms));
                                            actor.countdown_end = None;
                                            let to_act_uid = actor.table.seats.get(actor.table.to_act_idx).and_then(|s| s.user_id.clone());
                                            let ms_left = actor.action_deadline.map(|dl| dl.saturating_duration_since(Instant::now()).as_millis() as u64);
                                            let _ = actor.broadcast(&ServerEvent::TableSnapshot { table: actor.table.clone(), ready: actor.ready_status.clone(), to_act_uid, ms_left });
                                        } else {
                                            let ms_left = end.saturating_duration_since(Instant::now()).as_millis() as u64;
                                            let _ = actor.broadcast(&ServerEvent::GameStartCountdown { table_id: actor.table_id.clone(), ms_left });
                                        }
                                    }
                                } else {
                                    actor.countdown_end = None;
                                }
                            }
                        }
                    }
                    ActorMsg::Client(ClientAction::Join { table_id, buy_in, client_msg_id }) => {
                        info!(table_id=%table_id, buy_in, client_msg_id, "player_joined");
                        // sit the player with a synthetic user id derived from client_msg_id
                        let stack = if actor.config.starting_stack > 0 { actor.config.starting_stack } else { buy_in };
                        let _ = actor.table.sit(client_msg_id.clone(), stack);
                        actor.rebuys_left.entry(client_msg_id.clone()).or_insert(actor.config.rebuy_hands);
                        actor.ready_status.insert(client_msg_id.clone(), false);
                        if actor.table.active_player_count() >= 2 && actor.state.street.is_none() {
                            if actor.room_end_at.map_or(true, |end| Instant::now() < end) {
                                actor.table.start_hand(&mut actor.deck);
                                actor.action_deadline = Some(Instant::now() + Duration::from_millis(actor.config.action_time_ms));
                            } else { let _ = actor.broadcast(&ServerEvent::RoomClosed { table_id: actor.table_id.clone() }); }
                        }
                        let _ = actor.broadcast(&ServerEvent::PlayerJoined { table_id });
                        let to_act_uid = actor.table.seats.get(actor.table.to_act_idx).and_then(|s| s.user_id.clone());
                        let ms_left = actor.action_deadline.map(|dl| dl.saturating_duration_since(Instant::now()).as_millis() as u64);
                        let _ = actor.broadcast(&ServerEvent::TableSnapshot { table: actor.table.clone(), ready: actor.ready_status.clone(), to_act_uid, ms_left });
                    }
                    ActorMsg::Client(ClientAction::JoinRoom { table_id, client_msg_id }) => {
                        let _ = actor.table.sit(client_msg_id.clone(), actor.config.starting_stack);
                        actor.rebuys_left.entry(client_msg_id.clone()).or_insert(actor.config.rebuy_hands);
                        actor.ready_status.insert(client_msg_id.clone(), false);
                        if actor.table.active_player_count() >= 2 && actor.state.street.is_none() {
                            if actor.room_end_at.map_or(true, |end| Instant::now() < end) {
                                actor.table.start_hand(&mut actor.deck);
                                actor.action_deadline = Some(Instant::now() + Duration::from_millis(actor.config.action_time_ms));
                            } else { let _ = actor.broadcast(&ServerEvent::RoomClosed { table_id: actor.table_id.clone() }); }
                        }
                        let _ = actor.broadcast(&ServerEvent::PlayerJoined { table_id });
                        let to_act_uid = actor.table.seats.get(actor.table.to_act_idx).and_then(|s| s.user_id.clone());
                        let ms_left = actor.action_deadline.map(|dl| dl.saturating_duration_since(Instant::now()).as_millis() as u64);
                        let _ = actor.broadcast(&ServerEvent::TableSnapshot { table: actor.table.clone(), ready: actor.ready_status.clone(), to_act_uid, ms_left });
                    }
                    ActorMsg::Client(ClientAction::Ready { table_id, client_msg_id, ready }) => {
                        actor.ready_status.insert(client_msg_id.clone(), ready);
                        let _ = actor.broadcast(&ServerEvent::PlayerReady { table_id, client_msg_id: client_msg_id.clone(), ready });
                        // countdown will be handled in Tick cycle
                    }
                    ActorMsg::Client(ClientAction::LeaveRoom { table_id, client_msg_id }) => {
                        if let Some(seat) = actor.table.seats.iter_mut().find(|s| s.user_id.as_deref() == Some(&client_msg_id)) {
                            if actor.table.state.street.is_none() {
                                *seat = poker_ws::game::Seat::empty();
                                actor.ready_status.remove(&client_msg_id);
                            } else {
                                seat.sitting_out = true;
                                actor.ready_status.insert(client_msg_id.clone(), false);
                            }
                            actor.countdown_end = None; // any leave cancels countdown
                            let _ = actor.broadcast(&ServerEvent::PlayerLeft { table_id, client_msg_id });
                            let to_act_uid = actor.table.seats.get(actor.table.to_act_idx).and_then(|s| s.user_id.clone());
                            let ms_left = actor.action_deadline.map(|dl| dl.saturating_duration_since(Instant::now()).as_millis() as u64);
                            let _ = actor.broadcast(&ServerEvent::TableSnapshot { table: actor.table.clone(), ready: actor.ready_status.clone(), to_act_uid, ms_left });
                        }
                    }
                    ActorMsg::Client(ClientAction::Rebuy { table_id: _, client_msg_id }) => {
                        if actor.state.street.is_none() {
                            let entry = actor.rebuys_left.entry(client_msg_id.clone()).or_insert(actor.config.rebuy_hands);
                            if *entry > 0 {
                                if let Some(seat) = actor.table.seats.iter_mut().find(|s| s.user_id.as_deref() == Some(&client_msg_id)) {
                                    if seat.stack < actor.config.starting_stack { seat.stack = actor.config.starting_stack; *entry -= 1; 
                                        let to_act_uid = actor.table.seats.get(actor.table.to_act_idx).and_then(|s| s.user_id.clone());
                                        let ms_left = actor.action_deadline.map(|dl| dl.saturating_duration_since(Instant::now()).as_millis() as u64);
                                        let _ = actor.broadcast(&ServerEvent::TableSnapshot { table: actor.table.clone(), ready: actor.ready_status.clone(), to_act_uid, ms_left }); 
                                    }
                                }
                            }
                        }
                    }
                    ActorMsg::Client(ClientAction::Action { table_id, hand_id, action, amount, client_msg_id }) => {
                        info!(table_id=%table_id, hand_id=%hand_id, action=%action, amount=?amount, client_msg_id, "action_received");
                        let outcome = actor.table.apply_action_by_user(&client_msg_id, &action, amount);
                        match outcome {
                            Ok(ApplyOutcome::Continue) => {}
                            Ok(ApplyOutcome::NextStreet) => {
                                actor.table.next_street(&mut actor.deck);
                                actor.action_deadline = Some(Instant::now() + Duration::from_millis(actor.config.action_time_ms));
                                if actor.table.state.street == Some(poker_ws::Street::Showdown) {
                                    actor.table.showdown_and_payout();
                                    if actor.table.active_player_count() >= 2 && actor.room_end_at.map_or(true, |end| Instant::now() < end) {
                                        actor.table.start_hand(&mut actor.deck);
                                        actor.action_deadline = Some(Instant::now() + Duration::from_millis(actor.config.action_time_ms));
                                    } else if actor.room_end_at.is_some() { let _ = actor.broadcast(&ServerEvent::RoomClosed { table_id: actor.table_id.clone() }); }
                                }
                            },
                            Ok(ApplyOutcome::HandEnded) => { /* next hand will start on next join or by timer */ }
                            Err(e) => { let _ = actor.broadcast(&ServerEvent::Error{ message: e }); }
                        }
                        let _ = actor.broadcast(&ServerEvent::ActionAck { table_id, hand_id, action });
                        let to_act_uid = actor.table.seats.get(actor.table.to_act_idx).and_then(|s| s.user_id.clone());
                        let ms_left = actor.action_deadline.map(|dl| dl.saturating_duration_since(Instant::now()).as_millis() as u64);
                        let _ = actor.broadcast(&ServerEvent::TableSnapshot { table: actor.table.clone(), ready: actor.ready_status.clone(), to_act_uid, ms_left });
                    }
                }
            }
        });
        tx_return
    }

    fn broadcast(&mut self, evt: &ServerEvent) -> Result<(), ()> {
        let msg = serde_json::to_string(evt).map_err(|_| ())?;
        self.subscribers.retain(|tx| tx.send(WsMessage::Text(msg.clone())).is_ok());
        Ok(())
    }
}

type TableMap = Arc<Mutex<HashMap<String, mpsc::UnboundedSender<ActorMsg>>>>;

async fn ws_handler(ws: WebSocketUpgrade, State(tables): State<TableMap>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, tables))
}

async fn handle_socket(socket: WebSocket, tables: TableMap) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (client_tx, mut client_rx) = mpsc::unbounded_channel::<WsMessage>();

    // writer task
    let writer = tokio::spawn(async move {
        while let Some(msg) = client_rx.recv().await {
            if ws_tx.send(msg).await.is_err() { break; }
        }
    });

    // subscribe to a table lazily on first client message containing table_id
    while let Some(Ok(Message::Text(text))) = ws_rx.next().await {
        if let Ok(cmd) = serde_json::from_str::<ClientAction>(&text) {
            match &cmd {
                ClientAction::CreateRoom { table_id, config, client_msg_id } => {
                    let room_id = table_id.clone().unwrap_or_else(|| {
                        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
                        format!("r{:x}", now & 0xfffff)
                    });
                    let tx = {
                        let mut map = tables.lock().await;
                        if !map.contains_key(&room_id) {
                            let tx = TableActor::spawn_with_config(room_id.clone(), client_msg_id.clone(), config.clone());
                            map.insert(room_id.clone(), tx.clone());
                            tx
                        } else { map.get(&room_id).unwrap().clone() }
                    };
                    let _ = tx.send(ActorMsg::Subscribe(client_tx.clone()));
                    let _ = client_tx.send(WsMessage::Text(serde_json::to_string(&ServerEvent::RoomCreated { table_id: room_id }).unwrap()));
                }
                _ => {
                    let table_id = match &cmd {
                        ClientAction::Join { table_id, .. } => table_id,
                        ClientAction::Action { table_id, .. } => table_id,
                        ClientAction::JoinRoom { table_id, .. } => table_id,
                        ClientAction::Rebuy { table_id, .. } => table_id,
                        ClientAction::Ready { table_id, .. } => table_id,
                        ClientAction::LeaveRoom { table_id, .. } => table_id,
                        ClientAction::CreateRoom { .. } => unreachable!(),
                    }.clone();
                    let tx = {
                        let mut map = tables.lock().await;
                        map.entry(table_id.clone()).or_insert_with(|| TableActor::spawn(table_id.clone())).clone()
                    };
                    let _ = tx.send(ActorMsg::Subscribe(client_tx.clone()));
                    let _ = tx.send(ActorMsg::Client(cmd));
                }
            }
        } else {
            let _ = client_tx.send(WsMessage::Text("{\"type\":\"error\",\"message\":\"bad_json\"}".into()));
        }
    }

    let _ = writer.await;
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    let tables: TableMap = Arc::new(Mutex::new(HashMap::new()));
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .nest_service("/", ServeDir::new("public"))
        .with_state(tables);

    let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();
    info!(%addr, "starting server");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}



