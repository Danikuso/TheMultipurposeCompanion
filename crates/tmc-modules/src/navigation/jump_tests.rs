use super::*;
use crate::data::{StarSpecial, fsd_table};
use tmc_model::events::{Engineering, FuelCapacity, LoadoutModule, Modifier};

fn spec(bonus: f64) -> FsdSpec {
    FsdSpec {
        optimal_mass: 1050.0,
        max_fuel_per_jump: 5.0,
        fuel_mul: 0.012,
        fuel_power: 2.45,
        guardian_bonus_ly: bonus,
        kind: FsdKind::Standard,
    }
}

fn mass(unladen: f64, fuel: f64) -> ShipMass {
    ShipMass {
        unladen,
        fuel_main: fuel,
        fuel_reserve: 0.0,
        cargo: 0.0,
    }
}

fn module(slot: &str, item: &str, mods: Vec<Modifier>) -> LoadoutModule {
    LoadoutModule {
        slot: Some(slot.into()),
        item: Some(item.into()),
        health: Some(1.0),
        engineering: (!mods.is_empty()).then(|| Engineering {
            modifiers: mods,
            ..Default::default()
        }),
    }
}

fn loadout(item: &str, mods: Vec<Modifier>, max_range: Option<f64>) -> Loadout {
    Loadout {
        ship_id: Some(7),
        unladen_mass: Some(468.0),
        max_jump_range: max_range,
        fuel_capacity: Some(FuelCapacity {
            main: Some(32.0),
            reserve: Some(0.5),
        }),
        modules: vec![module("FrameShiftDrive", item, mods)],
        ..Default::default()
    }
}

#[test]
fn range_matches_reference_value() {
    let r = jump_range(&spec(0.0), &mass(495.0, 5.0));
    assert!((r - 24.63).abs() < 0.01, "{r}");
}

#[test]
fn guardian_booster_adds_flat_bonus() {
    let r = jump_range(&spec(10.5), &mass(495.0, 5.0));
    assert!((r - 35.13).abs() < 0.01, "{r}");
}

#[test]
fn fuel_for_max_jump_equals_max_fuel() {
    let f = fuel_for_jump(&spec(0.0), 500.0, 24.63).unwrap();
    assert!((f - 5.0).abs() < 0.01, "{f}");
}

#[test]
fn jump_beyond_range_is_not_feasible() {
    assert_eq!(fuel_for_jump(&spec(0.0), 500.0, 30.0), None);
}

#[test]
fn engineered_optimal_mass_is_applied() {
    let m = Modifier {
        label: "FSDOptimalMass".into(),
        value: Some(1692.6),
        ..Default::default()
    };
    let lo = loadout("int_hyperdrive_size5_class5", vec![m], None);
    let f = fsd_from_loadout(&lo, fsd_table()).unwrap();
    assert_eq!(f.optimal_mass, 1692.6);
    assert_eq!(f.max_fuel_per_jump, 5.0);
}

#[test]
fn guardian_booster_in_loadout_is_applied() {
    let mut lo = loadout("int_hyperdrive_size5_class5", vec![], None);
    lo.modules
        .push(module("Slot09", "int_guardianfsdbooster_size5", vec![]));
    let f = fsd_from_loadout(&lo, fsd_table()).unwrap();
    assert_eq!(f.guardian_bonus_ly, 10.5);
}

#[test]
fn supercharge_multipliers() {
    assert_eq!(
        supercharge_multiplier(FsdKind::Standard, StarSpecial::Neutron),
        4.0
    );
    assert_eq!(
        supercharge_multiplier(FsdKind::Standard, StarSpecial::WhiteDwarf),
        1.5
    );
    assert_eq!(
        supercharge_multiplier(FsdKind::ScoMk2, StarSpecial::Neutron),
        6.0
    );
    assert_eq!(
        supercharge_multiplier(FsdKind::ScoMk2, StarSpecial::WhiteDwarf),
        3.0
    );
}

#[test]
fn formula_validated_against_game_max_jump_range() {
    let lo = loadout("int_hyperdrive_size5_class5", vec![], Some(24.63));
    let m = build_jump_model(&lo, fsd_table());
    assert_eq!(m.source, RangeSource::Formula);
    assert_eq!(m.laden_ref_mass, Some(500.0));
    let r = m.range_now(&mass(468.0, 32.0)).unwrap();
    assert!((r - 24.63).abs() < 0.01, "{r}");
}

#[test]
fn unknown_fsd_item_falls_back_to_scaled_max_jump_range() {
    let lo = loadout("int_hyperdrive_size9_class9_future", vec![], Some(40.0));
    let m = build_jump_model(&lo, fsd_table());
    assert_eq!(m.source, RangeSource::Scaled);
    assert!(m.fsd.is_none());
    let empty = m.range_now(&mass(468.0, 32.0)).unwrap();
    assert!((empty - 40.0).abs() < 1e-9);
    let laden = ShipMass {
        cargo: 100.0,
        ..mass(468.0, 32.0)
    };
    let r = m.range_now(&laden).unwrap();
    assert!((r - 40.0 * 500.0 / 600.0).abs() < 1e-9, "{r}");
}

#[test]
fn mismatching_game_range_falls_back_to_scaled_keeping_fsd() {
    let lo = loadout("int_hyperdrive_size5_class5", vec![], Some(30.0));
    let m = build_jump_model(&lo, fsd_table());
    assert_eq!(m.source, RangeSource::Scaled);
    assert!(m.fsd.is_some());
}

#[test]
fn unknown_modifier_is_ignored_and_validation_decides() {
    let m = Modifier {
        label: "FSDFancyNew".into(),
        value: Some(1.0),
        ..Default::default()
    };
    let with = fsd_from_loadout(
        &loadout("int_hyperdrive_size5_class5", vec![m], None),
        fsd_table(),
    )
    .unwrap();
    let without = fsd_from_loadout(
        &loadout("int_hyperdrive_size5_class5", vec![], None),
        fsd_table(),
    )
    .unwrap();
    assert_eq!(with.optimal_mass, 1050.0);
    assert_eq!(with, without);
}

#[test]
fn unclamped_range_exceeds_clamped_when_tank_above_max_fuel() {
    let m = mass(468.0, 32.0);
    let c = jump_range_with(&spec(0.0), &m, true);
    assert!((c - 24.63).abs() < 0.01, "{c}");
    assert!(jump_range_with(&spec(0.0), &m, false) > 25.63);
}

#[test]
fn loadout_without_fsd_hides_fuel_after_jump() {
    let lo = Loadout {
        modules: vec![module("PowerPlant", "int_powerplant_size5_class5", vec![])],
        ..Default::default()
    };
    let m = build_jump_model(&lo, fsd_table());
    assert_eq!(m.source, RangeSource::Unknown);
    assert_eq!(m.range_now(&mass(468.0, 32.0)), None);
}
