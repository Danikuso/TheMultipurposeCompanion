//! Diagnóstico verbose del actor (solo compilaciones debug).
//!
//! Volumen: en replay/importación solo se emite un registro compacto por evento y el
//! estado completo en los primeros 32 eventos, luego cada 1000 y al terminar el replay.
//! En vivo hay verbosidad completa. Los topics se emiten completos (redactados) solo
//! si cambiaron y con un tope por evento.
use super::*;
use std::time::Instant;
use tmc_model::diagnostics::{DiagnosticContext as Ctx, json};

const FULL_FIRST: u64 = 32;
const FULL_EVERY: u64 = 1000;
const TOPIC_CAP: u32 = 8;

#[derive(Default)]
pub(super) struct ActorDiag {
    replay_events: u64,
    in_replay: bool,
    full: bool,
    topic_budget: u32,
    topics_changed: u32,
    topics_suppressed: u32,
    requests: u32,
    store_tasks: u32,
    alerts: u32,
    relevance_changes: u32,
}

pub(super) struct Begin {
    started: Instant,
    previous: Option<Value>,
}

fn kind(event: &SourceEvent) -> u8 {
    match event {
        SourceEvent::Journal { .. } => 0,
        SourceEvent::Status(_) => 1,
        SourceEvent::Companion(_) => 2,
        SourceEvent::GameProcess { .. } => 3,
        SourceEvent::Theme(_) => 4,
        SourceEvent::ImportProgress { .. } => 5,
        SourceEvent::Net(_) => 6,
        SourceEvent::Action { .. } => 7,
        SourceEvent::Settings(_) => 8,
        SourceEvent::Stored { .. } => 9,
    }
}

fn relevance_level(relevance: Relevance) -> u8 {
    match relevance {
        Relevance::Hidden => 0,
        Relevance::Normal => 1,
        Relevance::Active => 2,
        Relevance::Urgent => 3,
    }
}

impl Actor {
    pub(super) fn diag_begin(&mut self, event: &SourceEvent) -> Begin {
        let journal_replaying = match event {
            SourceEvent::Journal { replaying, .. } => Some(*replaying),
            _ => None,
        };
        if journal_replaying == Some(false) && self.diag.in_replay {
            self.diag_replay_finished();
        }
        let full = if journal_replaying == Some(true) {
            self.diag.in_replay = true;
            self.diag.replay_events += 1;
            let n = self.diag.replay_events;
            n <= FULL_FIRST || n.is_multiple_of(FULL_EVERY)
        } else {
            true
        };
        let diag = &mut self.diag;
        diag.full = full;
        diag.topic_budget = if full { TOPIC_CAP } else { 0 };
        diag.topics_changed = 0;
        diag.topics_suppressed = 0;
        diag.requests = 0;
        diag.store_tasks = 0;
        diag.alerts = 0;
        diag.relevance_changes = 0;
        Begin {
            started: Instant::now(),
            previous: full.then(|| value(&self.state)),
        }
    }

    pub(super) fn diag_end(&mut self, event: &SourceEvent, begin: Begin) {
        let duration_us = begin.started.elapsed().as_micros() as u64;
        let context = match event {
            SourceEvent::Journal { ev, .. } => Ctx::Journal { event: &ev.name },
            _ => Ctx::State,
        };
        let diag = &self.diag;
        tmc_model::diagnostic!(
            context,
            "actor.event.processed",
            json!({
                "kind": kind(event),
                "replaying": matches!(event, SourceEvent::Journal { replaying: true, .. }),
                "full": diag.full,
                "duration_us": duration_us,
                "topics_changed": diag.topics_changed,
                "topics_suppressed": diag.topics_suppressed,
                "relevance_changes": diag.relevance_changes,
                "requests": diag.requests,
                "store_tasks": diag.store_tasks,
                "alerts": diag.alerts,
                "modules": self.modules.len(),
                "seq": self.seq,
            })
        );
        if let Some(previous) = begin.previous {
            let next = value(&self.state);
            let payload = if previous == next {
                json!({"changed": false, "state": previous})
            } else {
                json!({"changed": true, "previous": previous, "next": next})
            };
            tmc_model::diagnostic!(Ctx::State, "actor.state.transition", payload);
        }
    }

    pub(super) fn diag_replay_finished(&mut self) {
        if !self.diag.in_replay {
            return;
        }
        self.diag.in_replay = false;
        let events = std::mem::take(&mut self.diag.replay_events);
        tmc_model::diagnostic!(
            Ctx::State,
            "actor.replay.finished",
            json!({"events": events, "state": value(&self.state)})
        );
    }

    pub(super) fn diag_topic(&mut self, topic: &str, topic_value: &Value) {
        self.diag.topics_changed += 1;
        if self.diag.topic_budget > 0 {
            self.diag.topic_budget -= 1;
            tmc_model::diagnostic!(
                Ctx::Topic { name: topic },
                "actor.topic.changed",
                topic_value
            );
        } else {
            self.diag.topics_suppressed += 1;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn diag_module(
        &mut self,
        id: &str,
        started: Instant,
        previous: Relevance,
        next: Relevance,
        refreshed: usize,
        requests: usize,
        store_tasks: usize,
        alerts: usize,
    ) {
        let diag = &mut self.diag;
        diag.requests += requests as u32;
        diag.store_tasks += store_tasks as u32;
        diag.alerts += alerts as u32;
        if previous != next {
            diag.relevance_changes += 1;
        }
        if !diag.full {
            return;
        }
        tmc_model::diagnostic!(
            Ctx::State,
            "actor.module.processed",
            json!({
                "module": id,
                "duration_us": started.elapsed().as_micros() as u64,
                "refreshed_topics": refreshed,
                "relevance_previous": relevance_level(previous),
                "relevance_next": relevance_level(next),
                "requests": requests,
                "store_tasks": store_tasks,
                "alerts": alerts,
            })
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmc_model::diagnostics::capture::{Record, capture};
    use tmc_model::events::parse_line;

    fn journal(name: &str, extra: &str, replaying: bool) -> SourceEvent {
        SourceEvent::Journal {
            ev: parse_line(&format!(
                r#"{{"timestamp":"2026-10-17T18:00:00Z","event":"{name}"{extra}}}"#
            ))
            .unwrap(),
            file: "Journal.synthetic.log".into(),
            line: 1,
            replaying,
        }
    }

    fn actor() -> Actor {
        Actor::new(vec![], None, broadcast::channel(16).0)
    }

    fn run(events: Vec<SourceEvent>) -> Vec<Record> {
        capture(|| {
            let mut actor = actor();
            for event in events {
                actor.apply(event);
            }
            actor.flush();
        })
        .1
    }

    const LOAD: &str = r#","Commander":"CANARY-CMDR","FID":"F-CANARY-FID","Ship":"python","ShipName":"CANARY-SHIP","ShipIdent":"CANARY-ID""#;

    #[test]
    fn replay_volume_is_bounded_and_state_is_sampled() {
        let mut events = vec![journal("LoadGame", LOAD, true)];
        events.extend((1..2100).map(|_| journal("Shutdown", "", true)));
        events.push(journal("Shutdown", "", false));
        let records = run(events);
        let count = |code: &str| records.iter().filter(|r| r.code == code).count();
        assert_eq!(count("actor.event.processed"), 2101);
        // 32 primeros + eventos 1000 y 2000 + el evento en vivo que cierra el replay.
        assert_eq!(count("actor.state.transition"), 32 + 2 + 1);
        assert_eq!(count("actor.replay.finished"), 1);
        assert!(count("actor.topic.changed") < 32 * 8 + 3 * 8);
        assert!(records.iter().all(|r| !r.payload.contains("CANARY")));
    }

    #[test]
    fn live_events_are_fully_traced_with_redacted_topics() {
        let records = run(vec![journal("LoadGame", LOAD, false)]);
        let codes: Vec<_> = records.iter().map(|r| r.code.as_str()).collect();
        assert!(codes.contains(&"actor.event.processed"));
        assert!(codes.contains(&"actor.state.transition"));
        assert!(codes.contains(&"actor.topic.changed"));
        let cmdr = records
            .iter()
            .find(|r| r.code == "actor.topic.changed" && r.schema == "core.cmdr")
            .expect("core.cmdr topic");
        assert_eq!(cmdr.context, "topic");
        assert!(records.iter().all(|r| !r.payload.contains("CANARY")));
        assert!(
            records
                .iter()
                .any(|r| r.code == "actor.event.processed" && r.schema == "LoadGame")
        );
    }

    #[test]
    fn same_input_gives_same_decisions_and_topics() {
        let events = || {
            vec![
                journal("LoadGame", LOAD, true),
                journal("Shutdown", "", true),
                journal("LoadGame", LOAD, false),
            ]
        };
        let shape = |records: Vec<Record>| {
            records
                .into_iter()
                .map(|r| {
                    let payload: Value = serde_json::from_str(&r.payload).unwrap();
                    // La duración es la única magnitud no determinista.
                    let stable = if r.code == "actor.event.processed" {
                        let mut p = payload;
                        p["duration_us"] = Value::Null;
                        p
                    } else if r.code == "actor.module.processed" {
                        Value::Null
                    } else {
                        payload
                    };
                    (r.context, r.schema, r.code, stable)
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(shape(run(events())), shape(run(events())));
    }
}
