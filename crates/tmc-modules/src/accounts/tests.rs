//! Tests del módulo `accounts` v2 (sin contadores ficticios).

use super::*;
use chrono::{DateTime, Utc};
use serde_json::json;
use tmc_model::accounts::CapiStatus;
use tmc_model::events::{Commander, EventKind, JournalEvent, LocationInfo};
use tmc_model::state::Galaxy;
use tmc_model::sync::{
    CmdrRef, CommanderRow, EddnServiceStatus, KeyAssignment, KeyedServiceStatus, PendingByService,
    RosterReport, SyncReason, SyncReport, SyncService, SyncServiceStatus, SyncState,
};

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
fn journal_without_sync_report_keeps_counters_zero() {
    let mut module = AccountsModule::new();
    let mut ctx = make_ctx(false);
    let state = GameState {
        galaxy: Galaxy::Live,
        ..Default::default()
    };

    for kind in [
        EventKind::Commander(Commander {
            name: Some("Danikuso".to_string()),
            fid: Some("F12345".to_string()),
        }),
        make_fsd_jump(),
        EventKind::Market,
        EventKind::Rank(Default::default()),
    ] {
        let ev = make_journal_event(kind, Utc::now());
        let changed = module.on_event(&ev, &state, &mut ctx);
        assert!(
            changed.is_empty(),
            "journal alone must not mutate accounts.status"
        );
    }

    let status = module.status();
    assert_eq!(status.inara.status.state, SyncState::Disabled);
    assert_eq!(status.inara.status.reason, Some(SyncReason::NoKey));
    assert_eq!(status.edsm.status.state, SyncState::Disabled);
    assert_eq!(status.edsm.status.reason, Some(SyncReason::NoKey));
    assert_eq!(status.eddn.status.state, SyncState::Disabled);
    assert_eq!(status.eddn.status.reason, Some(SyncReason::SwitchOff));
    for svc in [
        &status.inara.status,
        &status.edsm.status,
        &status.eddn.status,
    ] {
        assert_eq!(svc.pending, 0);
        assert_eq!(svc.sent_total, 0);
        assert_eq!(svc.sent_session, 0);
        assert_eq!(svc.rejected_total, 0);
        assert!(svc.last_confirmed_at.is_none());
    }
}

#[test]
fn sync_now_does_not_touch_counters() {
    let mut module = AccountsModule::new();
    let mut ctx = make_ctx(false);
    let state = GameState {
        galaxy: Galaxy::Live,
        ..Default::default()
    };

    let sync_now = SourceEvent::Action {
        name: "accounts.sync_now".to_string(),
        args: json!({}),
    };
    let changed = module.on_event(&sync_now, &state, &mut ctx);
    assert!(changed.is_empty());
    assert_eq!(module.status().inara.status.sent_total, 0);
    assert!(module.status().inara.status.last_confirmed_at.is_none());
    assert_eq!(module.status().edsm.status.sent_total, 0);
    assert!(module.status().edsm.status.last_confirmed_at.is_none());
}

#[test]
fn sync_report_updates_only_its_service() {
    let mut module = AccountsModule::new();
    let mut ctx = make_ctx(false);
    let state = GameState::default();

    let edsm_report = SourceEvent::SyncReport(Box::new(SyncReport::Edsm(KeyedServiceStatus {
        status: SyncServiceStatus {
            state: SyncState::Connected,
            reason: None,
            pending: 1,
            sent_total: 3,
            sent_session: 3,
            last_confirmed_at: Some("2026-11-13T00:00:00Z".to_string()),
            ..Default::default()
        },
        has_key: true,
        key_hint: Some("k1".to_string()),
    })));
    let changed = module.on_event(&edsm_report, &state, &mut ctx);
    assert!(changed.contains(TOPIC_STATUS));
    assert_eq!(module.status().edsm.status.sent_total, 3);
    assert_eq!(module.status().edsm.status.state, SyncState::Connected);
    assert_eq!(module.status().inara.status.state, SyncState::Disabled);
    assert_eq!(module.status().inara.status.sent_total, 0);
    assert_eq!(module.status().eddn.status.state, SyncState::Disabled);

    // Roster report updates roster fields only.
    let roster_report = SourceEvent::SyncReport(Box::new(SyncReport::Roster(RosterReport {
        current_cmdr: Some(CmdrRef {
            cmdr_ref: "cmdr-1".to_string(),
            name: "Danikuso".to_string(),
        }),
        commanders: vec![CommanderRow {
            cmdr_ref: "cmdr-1".to_string(),
            name: "Danikuso".to_string(),
            inara_key: false,
            edsm_key: true,
            key_assignment: Some(KeyAssignment::Confirmed),
            inara_pending: 0,
            edsm_pending: 1,
            eddn_pending: 0,
        }],
        other_cmdrs_pending: PendingByService::default(),
        unassigned_keys: vec![SyncService::Inara],
    })));
    let changed_roster = module.on_event(&roster_report, &state, &mut ctx);
    assert!(changed_roster.contains(TOPIC_STATUS));
    assert_eq!(
        module
            .status()
            .current_cmdr
            .as_ref()
            .map(|c| c.name.as_str()),
        Some("Danikuso")
    );
    assert_eq!(module.status().unassigned_keys, vec![SyncService::Inara]);

    // accounts.set_settings only accepts capi_* keys and leaves service counters intact.
    let settings_ev = SourceEvent::Action {
        name: "accounts.set_settings".to_string(),
        args: json!({
            "inara_has_key": true,
            "eddn_enabled": true,
            "capi_connected": true,
            "capi_enabled": true,
            "capi_remote_ships_count": 4,
            "capi_commander_name": "Danikuso",
        }),
    };
    let changed_capi = module.on_event(&settings_ev, &state, &mut ctx);
    assert!(changed_capi.contains(TOPIC_STATUS));
    assert!(module.status().capi.connected);
    assert_eq!(module.status().capi.remote_ships_count, 4);
    assert!(!module.status().inara.has_key);
    assert!(!module.status().eddn.enabled);
}

fn contract_snapshot() -> serde_json::Value {
    let waiting_and_disabled = AccountsStatusTopic {
        current_cmdr: Some(CmdrRef {
            cmdr_ref: "cmdr-1".to_string(),
            name: "Danikuso".to_string(),
        }),
        inara: KeyedServiceStatus {
            status: SyncServiceStatus {
                state: SyncState::Waiting,
                reason: None,
                pending: 2,
                recovered_session: 0,
                sent_total: 0,
                sent_session: 0,
                rejected_total: 0,
                rejected_session: 0,
                expired_total: 0,
                omitted_total: 0,
                last_confirmed_at: None,
                next_attempt_at: None,
                recent_rejections: Vec::new(),
            },
            has_key: true,
            key_hint: Some("...9a2f".to_string()),
        },
        edsm: KeyedServiceStatus {
            status: SyncServiceStatus {
                state: SyncState::Disabled,
                reason: Some(SyncReason::NoKey),
                ..Default::default()
            },
            has_key: false,
            key_hint: None,
        },
        eddn: EddnServiceStatus {
            status: SyncServiceStatus {
                state: SyncState::Disabled,
                reason: Some(SyncReason::SwitchOff),
                ..Default::default()
            },
            enabled: false,
            paused_schemas: Vec::new(),
        },
        commanders: vec![CommanderRow {
            cmdr_ref: "cmdr-1".to_string(),
            name: "Danikuso".to_string(),
            inara_key: true,
            edsm_key: false,
            key_assignment: Some(KeyAssignment::Confirmed),
            inara_pending: 2,
            edsm_pending: 0,
            eddn_pending: 0,
        }],
        other_cmdrs_pending: PendingByService::default(),
        unassigned_keys: Vec::new(),
        capi: CapiStatus::default(),
    };
    json!({
        "accounts.status": waiting_and_disabled,
    })
}

#[test]
fn ui_contract_snapshot_is_up_to_date() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../ui/tests/fixtures/accounts-topics.json");
    let text = serde_json::to_string_pretty(&contract_snapshot()).unwrap() + "\n";
    if std::env::var_os("UPDATE_CONTRACTS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &text).unwrap();
        return;
    }
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        current.replace("\r\n", "\n"),
        text,
        "el contrato JSON de accounts.status cambió: regenéralo con UPDATE_CONTRACTS=1 cargo test -p tmc-modules ui_contract"
    );
}
