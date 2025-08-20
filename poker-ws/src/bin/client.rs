use futures::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;
use serde_json::Value;

#[tokio::main]
async fn main() {
    let url = std::env::args().nth(1).unwrap_or_else(|| "ws://127.0.0.1:8080/ws".to_string());
    // modes: host <client_id> | join <client_id> <room_id> | leave <client_id> <room_id> | rebuy <client_id> <room_id>
    let mode = std::env::args().nth(2).unwrap_or_else(|| "host".to_string());
    let client_id = std::env::args().nth(3).unwrap_or_else(|| format!("c-{}", std::process::id()));
    let room_arg = std::env::args().nth(4);
    let (mut ws, _resp) = connect_async(url.as_str()).await.expect("connect");
    let mut room_id: Option<String> = None;
    match mode.as_str() {
        "host" => {
            let create = serde_json::json!({
                "type": "create_room",
                "table_id": null,
                "config": {"small_blind":5,"big_blind":10,"starting_stack":1000,"rebuy_hands":2,"room_duration_sec":300,"action_time_ms":3000},
                "client_msg_id": client_id
            });
            ws.send(tokio_tungstenite::tungstenite::Message::Text(create.to_string())).await.unwrap();
        }
        "join" => {
            let rid = room_arg.expect("room_id required for join");
            room_id = Some(rid.clone());
            let join = serde_json::json!({"type":"join_room","table_id":rid,"client_msg_id":client_id});
            ws.send(tokio_tungstenite::tungstenite::Message::Text(join.to_string())).await.unwrap();
        }
        "leave" => {
            let rid = room_arg.expect("room_id required for leave");
            room_id = Some(rid.clone());
            let leave = serde_json::json!({"type":"leave_room","table_id":rid,"client_msg_id":client_id});
            ws.send(tokio_tungstenite::tungstenite::Message::Text(leave.to_string())).await.unwrap();
        }
        "rebuy" => {
            let rid = room_arg.expect("room_id required for rebuy");
            room_id = Some(rid.clone());
            let rebuy = serde_json::json!({"type":"rebuy","table_id":rid,"client_msg_id":client_id});
            ws.send(tokio_tungstenite::tungstenite::Message::Text(rebuy.to_string())).await.unwrap();
        }
        _ => {}
    }

    // read loop; when room_created arrives, set room_id and auto ready
    let mut reads = 0;
    while reads < 100 {
        if let Some(msg) = ws.next().await {
            let txt = msg.unwrap().to_string();
            println!("<- {}", txt);
            if let Ok(v) = serde_json::from_str::<Value>(&txt) {
                if v.get("type").and_then(|t| t.as_str()) == Some("room_created") {
                    if let Some(tid) = v.get("table_id").and_then(|x| x.as_str()) { room_id = Some(tid.to_string()); }
                    if let Some(rid) = &room_id {
                        let ready = serde_json::json!({"type":"ready","table_id":rid,"client_msg_id":client_id,"ready":true});
                        ws.send(tokio_tungstenite::tungstenite::Message::Text(ready.to_string())).await.unwrap();
                    }
                }
            }
        }
        reads += 1;
    }
}


