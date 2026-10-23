use super::*;
use chrono::{DateTime, TimeZone, Utc};
use tmc_core::{Module, ModuleCtx, Relevance};
use tmc_model::carrier::*;
use tmc_model::events::{EventKind, JournalEvent};
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
fn next_weekly_upkeep_time_computes_thursday_0700_utc() {
    // Wednesday 15:00 UTC -> next day Thursday 07:00 UTC
    let wednesday = Utc.with_ymd_and_hms(2026, 10, 21, 15, 0, 0).unwrap();
    let next_upkeep = next_weekly_upkeep_time(wednesday);
    assert_eq!(
        next_upkeep,
        Utc.with_ymd_and_hms(2026, 10, 22, 7, 0, 0).unwrap()
    );

    // Thursday 06:30 UTC -> today Thursday 07:00 UTC
    let thursday_early = Utc.with_ymd_and_hms(2026, 10, 22, 6, 30, 0).unwrap();
    let next_upkeep = next_weekly_upkeep_time(thursday_early);
    assert_eq!(
        next_upkeep,
        Utc.with_ymd_and_hms(2026, 10, 22, 7, 0, 0).unwrap()
    );

    // Thursday 07:00 UTC exactly -> 7 days later Thursday 07:00 UTC
    let thursday_exact = Utc.with_ymd_and_hms(2026, 10, 22, 7, 0, 0).unwrap();
    let next_upkeep = next_weekly_upkeep_time(thursday_exact);
    assert_eq!(
        next_upkeep,
        Utc.with_ymd_and_hms(2026, 10, 29, 7, 0, 0).unwrap()
    );

    // Friday 12:00 UTC -> next Thursday 07:00 UTC
    let friday = Utc.with_ymd_and_hms(2026, 10, 23, 12, 0, 0).unwrap();
    let next_upkeep = next_weekly_upkeep_time(friday);
    assert_eq!(
        next_upkeep,
        Utc.with_ymd_and_hms(2026, 10, 29, 7, 0, 0).unwrap()
    );
}

#[test]
fn carrier_stats_populates_overview_and_finances() {
    let mut carrier = Carrier::new();
    let mut ctx = ModuleCtx::new("carrier", false, 1);
    let state = GameState::default();
    let now = Utc.with_ymd_and_hms(2026, 10, 23, 10, 0, 0).unwrap();

    let stats = CarrierStats {
        carrier_id: 3700012345,
        callsign: "X7Z-99A".to_string(),
        name: "STARLIGHT CRUSADER".to_string(),
        docking_access: "all".to_string(),
        allow_notorious: false,
        fuel_level: 800,
        jump_range_curr: 500.0,
        jump_range_max: 500.0,
        pending_decommission: false,
        space_usage: CarrierSpaceUsage {
            total_capacity: 25000,
            crew: 500,
            cargo: 4500,
            cargo_space_reserved: 0,
            ship_packs: 0,
            module_packs: 0,
            free_space: 20000,
        },
        finance: CarrierFinanceInfo {
            carrier_balance: 250_000_000,
            reserve_balance: 50_000_000,
            available_balance: 200_000_000,
            reserve_percent: 20,
            tax_rate_rearm: None,
            tax_rate_refuel: None,
            tax_rate_repair: None,
        },
        crew: vec![CarrierCrewMember {
            crew_role: "Shipyard".to_string(),
            activated: true,
            enabled: true,
            crew_name: Some("Tech Chief".to_string()),
        }],
        ship_packs: vec![],
        module_packs: vec![],
    };

    let ev = make_journal_event(EventKind::CarrierStats(Box::new(stats)), now);
    let changed = carrier.on_event(&ev, &state, &mut ctx);

    assert!(changed.contains(TOPIC_OVERVIEW));
    assert!(changed.contains(TOPIC_FINANCES));

    let overview_val = carrier.topic_value(TOPIC_OVERVIEW).expect("overview value");
    assert_eq!(overview_val["carrier_id"], 3700012345u64);
    assert_eq!(overview_val["callsign"], "X7Z-99A");
    assert_eq!(overview_val["fuel_depot"], 800);
    assert_eq!(overview_val["cargo_free"], 20000);

    let finances_val = carrier.topic_value(TOPIC_FINANCES).expect("finances value");
    assert_eq!(finances_val["balance"], 250_000_000i64);
    assert_eq!(finances_val["weekly_upkeep"], 11_500_000i64); // 5M base + 6.5M shipyard
    assert_eq!(finances_val["maintenance_health"], "safe");
}

#[test]
fn carrier_jump_request_starts_countdown_and_calculates_remaining_seconds() {
    let mut carrier = Carrier::new();
    let mut ctx = ModuleCtx::new("carrier", false, 1);
    let state = GameState::default();
    let t0 = Utc.with_ymd_and_hms(2026, 10, 23, 10, 0, 0).unwrap();
    let departure = t0 + chrono::Duration::minutes(15);

    let jump_req = CarrierJumpRequest {
        carrier_id: 3700012345,
        system_name: "Colonia".to_string(),
        body: Some("Colonia 4".to_string()),
        system_address: Some(123456789),
        body_id: Some(4),
        departure_time: Some(departure),
    };

    let ev = make_journal_event(EventKind::CarrierJumpRequest(jump_req), t0);
    carrier.on_event(&ev, &state, &mut ctx);

    let val = carrier.topic_value(TOPIC_JUMP).expect("jump value");
    assert_eq!(val["status"], "countingdown");
    assert_eq!(val["destination_system"], "Colonia");
    assert_eq!(val["destination_body"], "Colonia 4");
    assert_eq!(val["seconds_remaining"], 900); // 15 min * 60 s

    // Clock advances: 13 minutes later (2 minutes left -> Lockdown phase)
    let t_lock = t0 + chrono::Duration::minutes(13);
    carrier.update_jump_clock(t_lock);

    let val = carrier.topic_value(TOPIC_JUMP).expect("jump value");
    assert_eq!(val["status"], "locked");
    assert_eq!(val["seconds_remaining"], 120);
}

#[test]
fn carrier_jump_cancellation_resets_status_to_idle() {
    let mut carrier = Carrier::new();
    let mut ctx = ModuleCtx::new("carrier", false, 1);
    let state = GameState::default();
    let t0 = Utc.with_ymd_and_hms(2026, 10, 23, 10, 0, 0).unwrap();

    let jump_req = CarrierJumpRequest {
        carrier_id: 3700012345,
        system_name: "Colonia".to_string(),
        body: None,
        system_address: None,
        body_id: None,
        departure_time: Some(t0 + chrono::Duration::minutes(15)),
    };
    carrier.on_event(
        &make_journal_event(EventKind::CarrierJumpRequest(jump_req), t0),
        &state,
        &mut ctx,
    );

    let cancel = CarrierJumpCancelled {
        carrier_id: 3700012345,
    };
    carrier.on_event(
        &make_journal_event(EventKind::CarrierJumpCancelled(cancel), t0),
        &state,
        &mut ctx,
    );

    let val = carrier.topic_value(TOPIC_JUMP).expect("jump value");
    assert_eq!(val["status"], "idle");
    assert_eq!(val["is_cancelled"], true);
}

#[test]
fn carrier_deposit_fuel_updates_depot_and_total() {
    let mut carrier = Carrier::new();
    let mut ctx = ModuleCtx::new("carrier", false, 1);
    let state = GameState::default();
    let now = Utc.with_ymd_and_hms(2026, 10, 23, 10, 0, 0).unwrap();

    let deposit = CarrierDepositFuel {
        carrier_id: 3700012345,
        amount: 200,
        total: 1000,
    };
    carrier.on_event(
        &make_journal_event(EventKind::CarrierDepositFuel(deposit), now),
        &state,
        &mut ctx,
    );

    let val = carrier.topic_value(TOPIC_OVERVIEW).expect("overview value");
    assert_eq!(val["fuel_depot"], 1000);
}

#[test]
fn carrier_relevance_hidden_when_no_carrier_and_active_when_jumping() {
    let mut carrier = Carrier::new();
    let state = GameState::default();
    assert_eq!(carrier.relevance(&state), Relevance::Hidden);

    carrier.carrier_id = Some(3700012345);
    assert_eq!(carrier.relevance(&state), Relevance::Normal);

    carrier.jump_status = CarrierJumpStatus::CountingDown;
    assert_eq!(carrier.relevance(&state), Relevance::Active);
}
