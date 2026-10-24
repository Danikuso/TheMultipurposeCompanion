use chrono::{DateTime, Duration, TimeZone, Utc};
use tmc_core::{Module, ModuleCtx, Relevance};
use tmc_model::crime::{
    CommitCrimeEvent, CrimeStatusTopic, CrimeType, PayBountiesEvent, PayFinesEvent,
};
use tmc_model::events::{Cargo, CargoItem, EventKind, JournalEvent};
use tmc_model::source::SourceEvent;
use tmc_model::state::GameState;
use tmc_model::status::{Flags, Status};

use super::*;

fn mock_status(timestamp: DateTime<Utc>, flags: Flags, legal_state: Option<&str>) -> Status {
    Status {
        timestamp,
        flags,
        flags2: tmc_model::status::Flags2::empty(),
        pips: None,
        fire_group: None,
        gui_focus: Default::default(),
        fuel: None,
        cargo: None,
        legal_state: legal_state.map(|s| s.to_string()),
        latitude: None,
        longitude: None,
        altitude: None,
        heading: None,
        body_name: None,
        planet_radius: None,
        balance: None,
        destination: None,
        oxygen: None,
        health: None,
        temperature: None,
        gravity: None,
        selected_weapon: None,
    }
}

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
fn status_updates_legal_state_and_docked() {
    let mut module = Crime::new();
    let state = GameState::default();
    let mut ctx = ModuleCtx::new("crime", false, 0);

    let now = Utc::now();
    let s = mock_status(now, Flags::DOCKED, Some("Wanted"));
    let changed = module.on_event(&SourceEvent::Status(s), &state, &mut ctx);

    assert!(changed.contains(TOPIC_CRIME_STATUS));
    assert_eq!(module.legal_state, "Wanted");
    assert!(module.is_docked);
}

#[test]
fn notoriety_decays_only_when_flying() {
    let mut module = Crime::new();
    module.notoriety = 2;

    // 1. Atracado durante 7200 segundos: la notoriedad NO decae
    module.is_docked = true;
    let decayed = module.advance_flight_time(7200.0);
    assert!(!decayed);
    assert_eq!(module.notoriety, 2);
    assert_eq!(module.flight_time_seconds, 0.0);

    // 2. En vuelo (desatracado): 7200 segundos hacen decaer la notoriedad a 1
    module.is_docked = false;
    let decayed = module.advance_flight_time(7200.0);
    assert!(decayed);
    assert_eq!(module.notoriety, 1);
    assert_eq!(module.flight_time_seconds, 0.0);

    // 3. Otros 7200 segundos en vuelo hacen decaer la notoriedad a 0
    let decayed = module.advance_flight_time(7200.0);
    assert!(decayed);
    assert_eq!(module.notoriety, 0);
    assert_eq!(module.decay_seconds_remaining(), 0);

    // 4. Con notoriedad en 0, no decae más
    let decayed = module.advance_flight_time(7200.0);
    assert!(!decayed);
    assert_eq!(module.notoriety, 0);
}

#[test]
fn commit_crime_and_pay_fines_workflow() {
    let mut module = Crime::new();
    let mut state = GameState::default();
    state.location.system_address = 123456;
    state.location.system = "Sol".to_string();
    let mut ctx = ModuleCtx::new("crime", false, 0);
    module.system_address = 123456;

    let time = Utc.with_ymd_and_hms(2026, 10, 24, 14, 0, 0).unwrap();

    // Delito menor: multa de 500 cr
    let crime_fine = CommitCrimeEvent {
        crime_type: CrimeType::RecklessWeaponsDischarge,
        faction: "Federal Security".to_string(),
        victim: None,
        fine: Some(500),
        bounty: None,
    };
    let je1 = make_journal_event(EventKind::CommitCrime(Box::new(crime_fine)), time);
    module.on_event(&je1, &state, &mut ctx);

    assert_eq!(module.total_fines, 500);
    assert_eq!(module.notoriety, 0);
    assert_eq!(module.factions["Federal Security"].fines, 500);

    // Delito de sangre: asesinato incrementa notoriedad y añade recompensa
    let crime_murder = CommitCrimeEvent {
        crime_type: CrimeType::Murder,
        faction: "The Dark Wheel".to_string(),
        victim: Some("Trader".to_string()),
        fine: None,
        bounty: Some(15000),
    };
    let je2 = make_journal_event(
        EventKind::CommitCrime(Box::new(crime_murder)),
        time + Duration::minutes(5),
    );
    module.on_event(&je2, &state, &mut ctx);

    assert_eq!(module.total_bounties, 15000);
    assert_eq!(module.notoriety, 1);
    assert_eq!(module.factions["The Dark Wheel"].bounties, 15000);

    // Pago de multas
    let pay_fines = PayFinesEvent {
        amount: 500,
        broker_percentage: Some(25.0),
        all_fines: true,
        faction: Some("Federal Security".to_string()),
        ship_id: Some(1),
    };
    let je3 = make_journal_event(
        EventKind::PayFines(Box::new(pay_fines)),
        time + Duration::minutes(10),
    );
    module.on_event(&je3, &state, &mut ctx);

    assert_eq!(module.total_fines, 0);
    assert_eq!(module.factions["Federal Security"].fines, 0);

    // Pago de recompensas
    let pay_bounties = PayBountiesEvent {
        amount: 15000,
        broker_percentage: Some(25.0),
        all_bounties: true,
        faction: None,
        ship_id: Some(1),
    };
    let je4 = make_journal_event(
        EventKind::PayBounties(Box::new(pay_bounties)),
        time + Duration::minutes(15),
    );
    module.on_event(&je4, &state, &mut ctx);

    assert_eq!(module.total_bounties, 0);
    assert_eq!(module.factions["The Dark Wheel"].bounties, 0);
}

#[test]
fn cargo_with_stolen_items_updates_topic() {
    let mut module = Crime::new();
    let mut state = GameState::default();
    state.location.system_address = 123456;
    let mut ctx = ModuleCtx::new("crime", false, 0);

    let cargo_ev = Cargo {
        vessel: Some("Ship".to_string()),
        count: Some(10),
        inventory: Some(vec![
            CargoItem {
                name: "gold".to_string(),
                name_localised: Some("Oro".to_string()),
                count: 5,
                stolen: Some(5),
                mission_id: None,
            },
            CargoItem {
                name: "iron".to_string(),
                name_localised: Some("Hierro".to_string()),
                count: 5,
                stolen: Some(0),
                mission_id: None,
            },
        ]),
    };

    let je = make_journal_event(EventKind::Cargo(cargo_ev), Utc::now());
    let changed = module.on_event(&je, &state, &mut ctx);

    assert!(changed.contains(TOPIC_CRIME_STATUS));
    assert_eq!(module.stolen_items.len(), 1);
    assert_eq!(module.stolen_items[0].name, "gold");
    assert_eq!(module.stolen_items[0].stolen, 5);
}

#[test]
fn module_relevance_and_snapshot() {
    let mut module = Crime::new();
    let mut state = GameState::default();

    // Sin ubicación: Hidden
    assert_eq!(module.relevance(&state), Relevance::Hidden);

    state.location.system_address = 123456;
    // Con ubicación y limpio: Normal
    assert_eq!(module.relevance(&state), Relevance::Normal);

    // Con notoriedad o multas: Active
    module.notoriety = 1;
    assert_eq!(module.relevance(&state), Relevance::Active);

    // Topic value serializa a CrimeStatusTopic válido
    let snap = module.topic_value(TOPIC_CRIME_STATUS).unwrap();
    let topic: CrimeStatusTopic = serde_json::from_value(snap).unwrap();
    assert_eq!(topic.notoriety, 1);
    assert_eq!(topic.legal_state, "Clean");
}
