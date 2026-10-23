use super::*;
use chrono::{DateTime, TimeZone, Utc};
use tmc_core::{Module, ModuleCtx, Relevance};
use tmc_model::events::{EventKind, JournalEvent};
use tmc_model::powerplay::*;
use tmc_model::source::SourceEvent;
use tmc_model::state::GameState;

fn make_journal_event(kind: EventKind, timestamp: DateTime<Utc>) -> SourceEvent {
    SourceEvent::Journal {
        ev: JournalEvent {
            timestamp,
            name: "Event".to_string(),
            kind,
            raw: serde_json::value::RawValue::from_string("{}".to_string()).unwrap(),
        },
        file: "journal.log".to_string(),
        line: 1,
        replaying: false,
    }
}

#[test]
fn powerplay_event_populates_status_and_calculates_rank_progress() {
    let mut module = PowerplayModule::new();
    let mut ctx = ModuleCtx::new("powerplay", false, 1);
    let state = GameState::default();
    let now = Utc.with_ymd_and_hms(2026, 10, 23, 10, 0, 0).unwrap();

    // Commander pledged to Aisling Duval at Rank 4 with 10,000 merits
    // (Rank 4 base = 5,000, Rank 5 target = 15,000, progress = (10000 - 5000)/(15000 - 5000) = 50%)
    let pp = Powerplay {
        power: "Aisling Duval".to_string(),
        rank: 4,
        merits: 10_000,
        votes: Some(150),
        time_pledged: Some(3 * 604_800), // 3 weeks
    };

    let ev = make_journal_event(EventKind::Powerplay(Box::new(pp)), now);
    let changed = module.on_event(&ev, &state, &mut ctx);

    assert!(changed.contains(TOPIC_STATUS));

    let val = module.topic_value(TOPIC_STATUS).expect("topic value");
    assert_eq!(val["is_pledged"], true);
    assert_eq!(val["power"], "Aisling Duval");
    assert_eq!(val["rank"], 4);
    assert_eq!(val["merits"], 10_000);
    assert_eq!(val["next_rank_target"], 15_000);
    assert_eq!(val["merits_needed"], 5_000);
    assert_eq!(val["progress_percent"], 50.0);
    assert_eq!(val["weeks_pledged"], 3);
}

#[test]
fn powerplay_merits_updates_session_and_session_rate() {
    let mut module = PowerplayModule::new();
    let mut ctx = ModuleCtx::new("powerplay", false, 1);
    let state = GameState::default();
    let t0 = Utc.with_ymd_and_hms(2026, 10, 23, 10, 0, 0).unwrap();

    let pp = Powerplay {
        power: "Aisling Duval".to_string(),
        rank: 1,
        merits: 100,
        votes: None,
        time_pledged: Some(604_800),
    };
    module.on_event(
        &make_journal_event(EventKind::Powerplay(Box::new(pp)), t0),
        &state,
        &mut ctx,
    );

    // After 30 minutes (1800 seconds), earn 600 merits (rate: 600 * 3600 / 1800 = 1200 merits/h)
    let t1 = t0 + chrono::Duration::minutes(30);
    let merits_ev = PowerplayMerits {
        power: "Aisling Duval".to_string(),
        merits_earned: 600,
        total_merits: 700,
        activity: Some("Combat".to_string()),
    };
    let changed = module.on_event(
        &make_journal_event(EventKind::PowerplayMerits(merits_ev), t1),
        &state,
        &mut ctx,
    );

    assert!(changed.contains(TOPIC_STATUS));
    assert!(changed.contains(TOPIC_SESSION));

    let sess = module.topic_value(TOPIC_SESSION).expect("session value");
    assert_eq!(sess["session_merits_earned"], 600);
    assert_eq!(sess["merits_per_hour"], 1200.0);

    let stat = module.topic_value(TOPIC_STATUS).expect("status value");
    assert_eq!(stat["merits"], 700);
}

#[test]
fn powerplay_leave_clears_pledge() {
    let mut module = PowerplayModule::new();
    let mut ctx = ModuleCtx::new("powerplay", false, 1);
    let state = GameState::default();
    let t0 = Utc.with_ymd_and_hms(2026, 10, 23, 10, 0, 0).unwrap();

    let pp = Powerplay {
        power: "Aisling Duval".to_string(),
        rank: 2,
        merits: 800,
        votes: None,
        time_pledged: Some(100),
    };
    module.on_event(
        &make_journal_event(EventKind::Powerplay(Box::new(pp)), t0),
        &state,
        &mut ctx,
    );

    let leave = PowerplayLeave {
        power: "Aisling Duval".to_string(),
    };
    module.on_event(
        &make_journal_event(EventKind::PowerplayLeave(leave), t0),
        &state,
        &mut ctx,
    );

    let val = module.topic_value(TOPIC_STATUS).expect("status value");
    assert_eq!(val["is_pledged"], false);
    assert_eq!(val["power"], serde_json::Value::Null);
    assert_eq!(val["rank"], 0);
}

#[test]
fn powerplay_relevance_hidden_when_unpledged() {
    let mut module = PowerplayModule::new();
    let state = GameState::default();

    assert_eq!(module.relevance(&state), Relevance::Hidden);

    module.power = Some("Aisling Duval".to_string());
    assert_eq!(module.relevance(&state), Relevance::Normal);
}
