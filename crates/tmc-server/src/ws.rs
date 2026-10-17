//! Protocolo WebSocket (/ws).

use crate::AppState;
use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use tmc_core::CoreMessage;
use tokio::sync::broadcast;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum ClientMessage {
    Subscribe { topics: Vec<String> },
    Action { name: String },
}

pub fn matches_pattern(topic: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix(".*") {
        return topic
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('.'));
    }
    topic == pattern
}

pub fn is_subscribed(topic: &str, subs: &Option<Vec<String>>) -> bool {
    match subs {
        None => true,
        Some(patterns) => patterns.iter().any(|p| matches_pattern(topic, p)),
    }
}

fn filter_topics(
    topics: BTreeMap<String, Value>,
    subs: &Option<Vec<String>>,
) -> BTreeMap<String, Value> {
    topics
        .into_iter()
        .filter(|(k, _)| is_subscribed(k, subs))
        .collect()
}

async fn send_json(socket: &mut WebSocket, val: &Value) -> bool {
    let Ok(text) = serde_json::to_string(val) else {
        return false;
    };
    socket.send(Message::Text(text.into())).await.is_ok()
}

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.core.subscribe();
    let initial_snapshot = state.core.snapshot().await;
    let locale = state
        .store
        .kv_get("locale")
        .unwrap_or_else(|| "es".to_owned());
    let topic_names: Vec<String> = initial_snapshot.keys().cloned().collect();

    let hello = json!({
        "type": "hello",
        "version": 1,
        "locale": locale,
        "topics": topic_names,
    });
    if !send_json(&mut socket, &hello).await {
        return;
    }

    let snap_msg = json!({
        "type": "snapshot",
        "topics": initial_snapshot,
    });
    if !send_json(&mut socket, &snap_msg).await {
        return;
    }

    let mut subscriptions: Option<Vec<String>> = None;

    loop {
        tokio::select! {
            incoming = socket.recv() => {
                let Some(Ok(msg)) = incoming else {
                    break;
                };
                if let Message::Text(text) = msg {
                    if let Ok(parsed) = serde_json::from_str::<ClientMessage>(&text) {
                        match parsed {
                            ClientMessage::Subscribe { topics } => {
                                subscriptions = Some(topics);
                                let full = state.core.snapshot().await;
                                let filtered = filter_topics(full, &subscriptions);
                                let reply = json!({
                                    "type": "snapshot",
                                    "topics": filtered,
                                });
                                if !send_json(&mut socket, &reply).await {
                                    break;
                                }
                            }
                            ClientMessage::Action { name } => {
                                let snapshot = state.core.snapshot().await;
                                let _ = state.host.run_action(&name, &snapshot);
                            }
                        }
                    }
                } else if matches!(msg, Message::Close(_)) {
                    break;
                }
            }
            core_event = rx.recv() => {
                match core_event {
                    Ok(CoreMessage::Update { seq, topics }) => {
                        let filtered = filter_topics(topics, &subscriptions);
                        if !filtered.is_empty() {
                            let val = serde_json::to_value(CoreMessage::Update {
                                seq,
                                topics: filtered,
                            }).unwrap_or_default();
                            if !send_json(&mut socket, &val).await {
                                break;
                            }
                        }
                    }
                    Ok(msg @ (CoreMessage::Alert(_) | CoreMessage::ImportProgress { .. })) => {
                        let val = serde_json::to_value(msg).unwrap_or_default();
                        if !send_json(&mut socket, &val).await {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        let full = state.core.snapshot().await;
                        let filtered = filter_topics(full, &subscriptions);
                        let reply = json!({
                            "type": "snapshot",
                            "topics": filtered,
                        });
                        if !send_json(&mut socket, &reply).await {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        }
    }
}
