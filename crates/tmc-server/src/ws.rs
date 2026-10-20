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
    Subscribe {
        topics: Vec<String>,
    },
    Action {
        name: String,
        #[serde(default)]
        args: Option<Value>,
    },
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

/// Envía el JSON; devuelve los bytes enviados o `None` si falló.
async fn send_json(socket: &mut WebSocket, val: &Value) -> Option<usize> {
    let text = serde_json::to_string(val).ok()?;
    let bytes = text.len();
    socket
        .send(Message::Text(text.into()))
        .await
        .ok()
        .map(|()| bytes)
}

/// Contadores agregados por conexión; se vuelcan como mucho una vez por segundo.
#[cfg(debug_assertions)]
struct Stats {
    id: u64,
    opened: tokio::time::Instant,
    window: tokio::time::Instant,
    updates: u64,
    alerts: u64,
    filtered: u64,
    bytes: u64,
}

#[cfg(debug_assertions)]
impl Stats {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let now = tokio::time::Instant::now();
        Self {
            id: NEXT.fetch_add(1, Ordering::Relaxed),
            opened: now,
            window: now,
            updates: 0,
            alerts: 0,
            filtered: 0,
            bytes: 0,
        }
    }

    /// Instante en que debe volcarse la ventana si hay eventos pendientes.
    fn deadline(&self) -> Option<tokio::time::Instant> {
        (self.updates + self.alerts + self.filtered > 0)
            .then(|| self.window + std::time::Duration::from_secs(1))
    }

    fn flush(&mut self, force: bool) {
        let elapsed = self.window.elapsed();
        if (!force && elapsed < std::time::Duration::from_secs(1))
            || (self.updates + self.alerts + self.filtered == 0)
        {
            return;
        }
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Http,
            "ws.broadcast.summary",
            tmc_model::diagnostics::json!({"conn": self.id, "updates": self.updates,
                "alerts": self.alerts, "filtered_out": self.filtered, "bytes": self.bytes,
                "window_ms": elapsed.as_millis() as u64})
        );
        self.window = tokio::time::Instant::now();
        self.updates = 0;
        self.alerts = 0;
        self.filtered = 0;
        self.bytes = 0;
    }
}

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    #[cfg(debug_assertions)]
    let mut stats = Stats::new();
    #[cfg(debug_assertions)]
    tmc_model::diagnostic!(
        tmc_model::diagnostics::DiagnosticContext::Http,
        "ws.connected",
        tmc_model::diagnostics::json!({"conn": stats.id})
    );
    #[cfg(debug_assertions)]
    let (reason, close_code) = run_socket(&mut socket, &state, &mut stats).await;
    #[cfg(not(debug_assertions))]
    let _ = run_socket(&mut socket, &state).await;
    #[cfg(debug_assertions)]
    {
        stats.flush(true);
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Http,
            "ws.closed",
            tmc_model::diagnostics::json!({"conn": stats.id, "reason": reason,
                "close_code": close_code,
                "duration_ms": stats.opened.elapsed().as_millis() as u64})
        );
    }
}

/// Motivos de cierre: 0 flujo terminado, 1 cierre del cliente, 2 envío fallido,
/// 3 canal del núcleo cerrado.
async fn run_socket(
    socket: &mut WebSocket,
    state: &AppState,
    #[cfg(debug_assertions)] stats: &mut Stats,
) -> (u8, u16) {
    let mut rx = state.core.subscribe();
    let initial_snapshot = state.core.snapshot().await;
    let locale = state
        .store
        .kv_get("locale")
        .unwrap_or_else(|| "es".to_owned());
    let topic_names: Vec<String> = initial_snapshot.keys().cloned().collect();

    #[allow(unused_mut)]
    let mut hello = json!({
        "type": "hello",
        "version": 1,
        "locale": locale,
        "topics": topic_names,
    });
    // Solo un backend de depuración anuncia la capacidad: la UI decide por ella.
    #[cfg(debug_assertions)]
    {
        hello["capabilities"] = json!({"diagnostics": true});
    }
    let Some(_hello_bytes) = send_json(socket, &hello).await else {
        return (2, 0);
    };
    tmc_model::diagnostic!(
        tmc_model::diagnostics::DiagnosticContext::Http,
        "ws.hello.sent",
        tmc_model::diagnostics::json!({"conn": stats.id, "topics": topic_names.len(),
            "bytes": _hello_bytes})
    );

    let snap_msg = json!({
        "type": "snapshot",
        "topics": initial_snapshot,
    });
    let Some(_snap_bytes) = send_json(socket, &snap_msg).await else {
        return (2, 0);
    };
    tmc_model::diagnostic!(
        tmc_model::diagnostics::DiagnosticContext::Http,
        "ws.snapshot.initial",
        tmc_model::diagnostics::json!({"conn": stats.id, "bytes": _snap_bytes})
    );

    let mut subscriptions: Option<Vec<String>> = None;

    loop {
        #[cfg(debug_assertions)]
        stats.flush(false);
        #[cfg(debug_assertions)]
        let flush_at = stats.deadline();
        #[cfg(not(debug_assertions))]
        let flush_at = std::hint::black_box(None::<tokio::time::Instant>);
        tokio::select! {
            () = tokio::time::sleep_until(flush_at.unwrap_or_else(tokio::time::Instant::now)),
                if flush_at.is_some() => {
                #[cfg(debug_assertions)]
                stats.flush(false);
            }
            incoming = socket.recv() => {
                let Some(Ok(msg)) = incoming else {
                    break (0, 0);
                };
                if let Message::Text(text) = msg {
                    if let Ok(parsed) = serde_json::from_str::<ClientMessage>(&text) {
                        match parsed {
                            ClientMessage::Subscribe { topics } => {
                                tmc_model::diagnostic!(
                                    tmc_model::diagnostics::DiagnosticContext::Http,
                                    "ws.subscribe",
                                    tmc_model::diagnostics::json!({"conn": stats.id,
                                        "patterns": topics.len(),
                                        "wildcard": topics.iter().any(|t| t == "*")})
                                );
                                subscriptions = Some(topics);
                                let full = state.core.snapshot().await;
                                let filtered = filter_topics(full, &subscriptions);
                                let reply = json!({
                                    "type": "snapshot",
                                    "topics": filtered,
                                });
                                let Some(_sent) = send_json(socket, &reply).await else {
                                    break (2, 0);
                                };
                                tmc_model::diagnostic!(
                                    tmc_model::diagnostics::DiagnosticContext::Http,
                                    "ws.snapshot.subscribed",
                                    tmc_model::diagnostics::json!({"conn": stats.id, "bytes": _sent})
                                );
                            }
                            ClientMessage::Action { name, args } => {
                                let snapshot = state.core.snapshot().await;
                                let args = args.unwrap_or(Value::Null);
                                let _ = crate::diag::run_action(
                                    state.host.as_ref(), &name, &args, &snapshot, true);
                            }
                        }
                    } else {
                        tmc_model::diagnostic!(
                            tmc_model::diagnostics::DiagnosticContext::Http,
                            "ws.client.invalid_message",
                            tmc_model::diagnostics::json!({"conn": stats.id, "bytes": text.len()})
                        );
                    }
                } else if let Message::Close(frame) = msg {
                    break (1, frame.map_or(0, |f| f.code));
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
                            let Some(sent) = send_json(socket, &val).await else {
                                break (2, 0);
                            };
                            #[cfg(debug_assertions)]
                            {
                                stats.updates += 1;
                                stats.bytes += sent as u64;
                            }
                            let _ = sent;
                        } else {
                            #[cfg(debug_assertions)]
                            {
                                stats.filtered += 1;
                            }
                        }
                    }
                    Ok(msg @ (CoreMessage::Alert(_) | CoreMessage::ImportProgress { .. })) => {
                        let val = serde_json::to_value(msg).unwrap_or_default();
                        let Some(sent) = send_json(socket, &val).await else {
                            break (2, 0);
                        };
                        #[cfg(debug_assertions)]
                        {
                            stats.alerts += 1;
                            stats.bytes += sent as u64;
                        }
                        let _ = sent;
                    }
                    Err(broadcast::error::RecvError::Lagged(_skipped)) => {
                        tmc_model::diagnostic!(
                            tmc_model::diagnostics::DiagnosticContext::Http,
                            "ws.backpressure.lagged",
                            tmc_model::diagnostics::json!({"conn": stats.id, "skipped": _skipped})
                        );
                        let full = state.core.snapshot().await;
                        let filtered = filter_topics(full, &subscriptions);
                        let reply = json!({
                            "type": "snapshot",
                            "topics": filtered,
                        });
                        if send_json(socket, &reply).await.is_none() {
                            break (2, 0);
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break (3, 0);
                    }
                }
            }
        }
    }
}
