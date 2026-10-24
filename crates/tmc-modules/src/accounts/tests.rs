//! Tests del módulo `accounts`.

use super::*;
use chrono::{DateTime, Utc};
use serde_json::json;
use tmc_model::events::{Commander, EventKind, JournalEvent, LocationInfo};

fn make_ctx(replaying: bool) -> ModuleCtx {
    ModuleCtx::new("accounts", replaying, 1)
}

fn make_journal_event(kind: EventKind, timestamp: DateTime<Utc>) -> SourceEvent {
    SourceEvent::Journal {
        ev: JournalEvent {
            timestamp,
            name: "Event".to_string(),
            kind,
            raw: serde_json::value::RawValue::from_string("{}".to_string()).unwrap(),
        },
        file: "Journal.01.log".to_string(),
        line: 1,
        replaying: false,
    }
}

fn make_fsd_jump() -> EventKind {
    EventKind::FsdJump(Box::new(LocationInfo {
        star_system: "Sol".to_string(),
        system_address: 12345,
        star_pos: [0.0, 0.0, 0.0],
        body: Some("Sol".to_string()),
        body_type: Some("Star".to_string()),
        docked: false,
        station_name: None,
        station_type: None,
        market_id: None,
        jump_dist: Some(15.0),
        fuel_used: Some(1.2),
        fuel_level: Some(30.0),
        body_id: Some(0),
        ..Default::default()
    }))
}

#[test]
fn default_module_topics_and_relevance() {
    let module = AccountsModule::new();
    assert_eq!(module.id(), "accounts");
    assert_eq!(module.topics(), vec![TOPIC_STATUS]);
    assert_eq!(module.relevance(&GameState::default()), Relevance::Normal);

    let val = module.topic_value(TOPIC_STATUS).unwrap();
    assert!(val.get("eddn").is_some());
    assert!(val.get("edsm").is_some());
    assert!(val.get("inara").is_some());
    assert!(val.get("capi").is_some());
}

#[test]
fn legacy_galaxy_blocks_all_transmissions() {
    let mut module = AccountsModule::new();
    let mut ctx = make_ctx(false);

    let state = GameState {
        galaxy: Galaxy::Legacy,
        ..Default::default()
    };

    let fsd = make_journal_event(make_fsd_jump(), Utc::now());

    let changed = module.on_event(&fsd, &state, &mut ctx);
    assert!(
        changed.is_empty(),
        "en Legacy no debe emitirse ningún cambio ni transmisión"
    );
    assert_eq!(module.status().eddn.messages_sent_session, 0);
}

#[test]
fn replaying_mode_does_not_count_session_messages() {
    let mut module = AccountsModule::new();
    let mut ctx = make_ctx(true); // Replaying histórico

    let state = GameState {
        galaxy: Galaxy::Live,
        ..Default::default()
    };

    let fsd = make_journal_event(make_fsd_jump(), Utc::now());

    let changed = module.on_event(&fsd, &state, &mut ctx);
    assert!(changed.is_empty());
    assert_eq!(module.status().eddn.messages_sent_session, 0);
}

#[test]
fn live_events_update_counters_and_timestamps() {
    let mut module = AccountsModule::new();
    let mut ctx = make_ctx(false);

    let state = GameState {
        galaxy: Galaxy::Live,
        ..Default::default()
    };

    // 1. Evento Commander
    let cmdr_ev = make_journal_event(
        EventKind::Commander(Commander {
            name: Some("Danikuso".to_string()),
            fid: Some("F12345".to_string()),
        }),
        Utc::now(),
    );
    let changed = module.on_event(&cmdr_ev, &state, &mut ctx);
    assert!(changed.contains(TOPIC_STATUS));
    assert_eq!(
        module.status().edsm.commander_name.as_deref(),
        Some("Danikuso")
    );
    assert_eq!(
        module.status().inara.commander_name.as_deref(),
        Some("Danikuso")
    );
    assert_eq!(
        module.status().capi.commander_name.as_deref(),
        Some("Danikuso")
    );
    assert_eq!(module.status().eddn.uploader_id, "TMC-Danikuso");

    // 2. Activar claves para EDSM e Inara
    let set_keys = SourceEvent::Action {
        name: "accounts.set_settings".to_string(),
        args: json!({
            "edsm_enabled": true,
            "edsm_has_key": true,
            "inara_enabled": true,
            "inara_has_key": true,
        }),
    };
    module.on_event(&set_keys, &state, &mut ctx);
    assert!(module.status().edsm.has_key);
    assert!(module.status().inara.has_key);

    // 3. Salto FSDJump
    let fsd = make_journal_event(make_fsd_jump(), Utc::now());
    let changed = module.on_event(&fsd, &state, &mut ctx);
    assert!(changed.contains(TOPIC_STATUS));
    assert_eq!(module.status().eddn.messages_sent_session, 1);
    assert!(module.status().eddn.last_upload_time.is_some());
    assert!(module.status().edsm.last_sync_time.is_some());
    assert_eq!(module.status().inara.pending_events, 1);

    // 4. Mercado (Market)
    let mkt = make_journal_event(EventKind::Market, Utc::now());
    let changed_mkt = module.on_event(&mkt, &state, &mut ctx);
    assert!(changed_mkt.contains(TOPIC_STATUS));
    assert_eq!(module.status().eddn.messages_sent_session, 2);

    // 5. Acción sync_now vacía el buffer de Inara
    let sync_now = SourceEvent::Action {
        name: "accounts.sync_now".to_string(),
        args: json!({}),
    };
    let changed_sync = module.on_event(&sync_now, &state, &mut ctx);
    assert!(changed_sync.contains(TOPIC_STATUS));
    assert_eq!(module.status().inara.pending_events, 0);
}
