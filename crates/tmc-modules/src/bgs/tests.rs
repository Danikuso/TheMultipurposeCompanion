use super::*;
use chrono::{DateTime, TimeZone, Utc};
use tmc_core::{Module, ModuleCtx, Relevance};
use tmc_model::events::{EventKind, FactionInfo, JournalEvent, LocationInfo, SystemFaction};
use tmc_model::net::{BgsTickInfo, NetOutcome, NetResponse, NetSource};
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
fn bgs_system_parses_factions_and_sorts_by_influence() {
    let mut module = Bgs::new();
    let mut ctx = ModuleCtx::new("bgs", false, 1);
    let state = GameState::default();
    let now = Utc.with_ymd_and_hms(2026, 10, 23, 10, 0, 0).unwrap();

    let loc = LocationInfo {
        star_system: "Barnard's Star".to_string(),
        system_address: 1234567,
        star_pos: [1.0, 2.0, 3.0],
        system_faction: Some(SystemFaction {
            name: Some("Barnard's Star Alliance".to_string()),
            faction_state: Some("Election".to_string()),
        }),
        factions: vec![
            FactionInfo {
                name: "Barnard's Star Corp".to_string(),
                faction_state: Some("None".to_string()),
                influence: Some(0.25),
                allegiance: Some("Independent".to_string()),
                government: Some("Corporate".to_string()),
                happiness_localised: Some("Happy".to_string()),
                active_states: vec![],
            },
            FactionInfo {
                name: "Barnard's Star Alliance".to_string(),
                faction_state: Some("Election".to_string()),
                influence: Some(0.55),
                allegiance: Some("Federation".to_string()),
                government: Some("Democracy".to_string()),
                happiness_localised: Some("Elated".to_string()),
                active_states: vec![],
            },
            FactionInfo {
                name: "Barnard Pirates".to_string(),
                faction_state: Some("CivilUnrest".to_string()),
                influence: Some(0.20),
                allegiance: Some("Independent".to_string()),
                government: Some("Anarchy".to_string()),
                happiness_localised: Some("Unhappy".to_string()),
                active_states: vec![],
            },
        ],
        ..Default::default()
    };

    let ev = make_journal_event(EventKind::FsdJump(Box::new(loc)), now);
    let changed = module.on_event(&ev, &state, &mut ctx);

    assert!(changed.contains(TOPIC_SYSTEM));

    let val = module.topic_value(TOPIC_SYSTEM).expect("system value");
    assert_eq!(val["star_system"], "Barnard's Star");
    assert_eq!(val["controlling_faction"], "Barnard's Star Alliance");
    assert_eq!(val["controlling_faction_state"], "Election");
    assert_eq!(val["factions_count"], 3);

    let factions = val["factions"].as_array().expect("factions array");
    // Sorted descending by influence: 55%, 25%, 20%
    assert_eq!(factions[0]["name"], "Barnard's Star Alliance");
    assert_eq!(factions[0]["influence_percent"], 55.0);
    assert_eq!(factions[0]["is_controlling"], true);

    assert_eq!(factions[1]["name"], "Barnard's Star Corp");
    assert_eq!(factions[1]["influence_percent"], 25.0);
    assert_eq!(factions[1]["is_controlling"], false);

    assert_eq!(factions[2]["name"], "Barnard Pirates");
    assert_eq!(factions[2]["influence_percent"], 20.0);
    assert_eq!(factions[2]["is_controlling"], false);
}

#[test]
fn bgs_tick_response_populates_tick_topic() {
    let mut module = Bgs::new();
    let mut ctx = ModuleCtx::new("bgs", false, 1);
    let state = GameState::default();
    let now = Utc.with_ymd_and_hms(2026, 10, 23, 10, 0, 0).unwrap();
    let tick_time = Utc.with_ymd_and_hms(2026, 10, 23, 4, 0, 0).unwrap(); // 6 hours ago

    let net_resp = NetResponse {
        id: 1,
        module: "bgs".to_string(),
        key: "bgs:tick".to_string(),
        source: NetSource::Edcd,
        fetched_at: now,
        outcome: NetOutcome::BgsTick(BgsTickInfo {
            latest_tick: tick_time,
            updated_at: now,
        }),
    };

    let ev = SourceEvent::Net(net_resp);
    let changed = module.on_event(&ev, &state, &mut ctx);

    assert!(changed.contains(TOPIC_TICK));

    module.last_now = Some(now);
    let val = module.topic_value(TOPIC_TICK).expect("tick value");
    assert_eq!(val["latest_tick"], "2026-10-23T04:00:00Z");
    assert_eq!(val["seconds_since_tick"], 6 * 3600);
    assert_eq!(val["hours_since_tick"], 6.0);
    assert_eq!(val["is_stale"], false);
}

#[test]
fn bgs_relevance_active_during_conflict_states() {
    let mut module = Bgs::new();
    let state = GameState::default();

    assert_eq!(module.relevance(&state), Relevance::Hidden);

    module.star_system = Some("Sol".to_string());
    assert_eq!(module.relevance(&state), Relevance::Normal);

    // War active
    module.factions = vec![FactionInfo {
        name: "Faction War".to_string(),
        faction_state: Some("War".to_string()),
        influence: Some(0.4),
        allegiance: None,
        government: None,
        happiness_localised: None,
        active_states: vec![],
    }];
    assert_eq!(module.relevance(&state), Relevance::Active);
}
