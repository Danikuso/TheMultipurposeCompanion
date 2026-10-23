use super::*;
use chrono::{DateTime, TimeZone, Utc};
use tmc_core::{Module, ModuleCtx, Relevance};
use tmc_model::colonisation::*;
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
fn colonisation_depot_event_populates_topic_and_calculates_trips() {
    let mut module = Colonisation::new();
    let mut ctx = ModuleCtx::new("colonisation", false, 1);
    let mut state = GameState::default();
    // Simulate player ship with 720t cargo capacity
    state.ship = Some(tmc_model::state::ShipState {
        cargo_capacity: Some(720),
        ..Default::default()
    });

    let now = Utc.with_ymd_and_hms(2026, 10, 23, 10, 0, 0).unwrap();

    let depot = ColonisationConstructionDepot {
        market_id: Some(3900012345),
        station_name: Some("Trailblazer Starport Alpha".to_string()),
        construction_type: Some("CoriolisStation".to_string()),
        construction_progress: 0.65,
        resources_required: vec![
            ColonisationResource {
                name: "Steel".to_string(),
                name_localised: Some("Acero".to_string()),
                required_amount: 10_000,
                provided_amount: 7_000, // 3,000 remaining -> ceil(3000 / 720) = 5 trips
            },
            ColonisationResource {
                name: "Titanium".to_string(),
                name_localised: Some("Titanio".to_string()),
                required_amount: 5_000,
                provided_amount: 4_000, // 1,000 remaining -> ceil(1000 / 720) = 2 trips
            },
        ],
    };

    let ev = make_journal_event(
        EventKind::ColonisationConstructionDepot(Box::new(depot)),
        now,
    );
    let changed = module.on_event(&ev, &state, &mut ctx);

    assert!(changed.contains(TOPIC_DEPOT));

    let val = module.topic_value(TOPIC_DEPOT).expect("topic value");
    assert_eq!(val["active_project"], true);
    assert_eq!(val["market_id"], 3900012345u64);
    assert_eq!(val["station_name"], "Trailblazer Starport Alpha");
    assert_eq!(val["construction_type"], "CoriolisStation");
    assert_eq!(val["total_required"], 15_000);
    assert_eq!(val["total_provided"], 11_000);
    assert_eq!(val["total_remaining"], 4_000);
    assert_eq!(val["cargo_capacity"], 720);
    assert_eq!(val["total_trips_remaining"], 6); // ceil(4000 / 720) = 6

    let resources = val["resources"].as_array().expect("resources array");
    assert_eq!(resources.len(), 2);
    assert_eq!(resources[0]["name"], "Steel");
    assert_eq!(resources[0]["remaining"], 3_000);
    assert_eq!(resources[0]["trips_remaining"], 5);
    assert_eq!(resources[1]["name"], "Titanium");
    assert_eq!(resources[1]["remaining"], 1_000);
    assert_eq!(resources[1]["trips_remaining"], 2);
}

#[test]
fn colonisation_contribution_updates_resource_and_trips() {
    let mut module = Colonisation::new();
    let mut ctx = ModuleCtx::new("colonisation", false, 1);
    let mut state = GameState::default();
    state.ship = Some(tmc_model::state::ShipState {
        cargo_capacity: Some(500),
        ..Default::default()
    });

    let t0 = Utc.with_ymd_and_hms(2026, 10, 23, 10, 0, 0).unwrap();

    let depot = ColonisationConstructionDepot {
        market_id: Some(3900012345),
        station_name: Some("Colony Depot".to_string()),
        construction_type: Some("Outpost".to_string()),
        construction_progress: 0.50,
        resources_required: vec![ColonisationResource {
            name: "Steel".to_string(),
            name_localised: Some("Acero".to_string()),
            required_amount: 1_000,
            provided_amount: 500, // 500 remaining -> 1 trip of 500t
        }],
    };
    module.on_event(
        &make_journal_event(
            EventKind::ColonisationConstructionDepot(Box::new(depot)),
            t0,
        ),
        &state,
        &mut ctx,
    );

    // Player contributes 500t of Steel
    let contrib = ColonisationContribution {
        market_id: Some(3900012345),
        commodity: "Steel".to_string(),
        commodity_localised: Some("Acero".to_string()),
        amount: 500,
        remaining_amount: Some(0),
        reward_credits: Some(250_000),
    };
    let changed = module.on_event(
        &make_journal_event(EventKind::ColonisationContribution(contrib), t0),
        &state,
        &mut ctx,
    );

    assert!(changed.contains(TOPIC_DEPOT));

    let val = module.topic_value(TOPIC_DEPOT).expect("topic value");
    assert_eq!(val["total_provided"], 1_000);
    assert_eq!(val["total_remaining"], 0);
    assert_eq!(val["total_trips_remaining"], 0);
    assert_eq!(val["progress_percent"], 100.0);
}

#[test]
fn colonisation_relevance_hidden_without_depot_active_when_docked() {
    let mut module = Colonisation::new();
    let state = GameState::default();

    assert_eq!(module.relevance(&state), Relevance::Hidden);

    module.active_depot = Some(ColonisationConstructionDepot {
        market_id: Some(123),
        station_name: Some("Depot".to_string()),
        construction_type: None,
        construction_progress: 0.1,
        resources_required: vec![],
    });
    assert_eq!(module.relevance(&state), Relevance::Normal);

    module.is_docked = true;
    assert_eq!(module.relevance(&state), Relevance::Active);
}
