//! Todo lo catalogado tiene nombre en los cinco idiomas (spec F4 §7).

use tmc_i18n::{I18n, LANGUAGES, keys};
use tmc_modules::data::{blueprints, engineers};

fn assert_in_all_languages(key: &str) {
    for lang in LANGUAGES {
        assert!(keys(lang).contains(key), "{lang}: missing {key}");
    }
}

#[test]
fn every_catalogued_blueprint_and_experimental_has_a_name_in_all_languages() {
    let catalog = blueprints();
    assert!(!catalog.blueprints.is_empty() && !catalog.experimentals.is_empty());
    for b in &catalog.blueprints {
        assert_in_all_languages(&format!("bp-{}", b.key));
    }
    for e in &catalog.experimentals {
        assert_in_all_languages(&format!("exp-{}", e.key));
    }
}

#[test]
fn unlock_keys_exist_in_engineering_data_ftl() {
    assert!(!engineers().is_empty());
    for e in engineers() {
        assert_in_all_languages(&e.unlock.requirement_key);
        if let Some(invite) = &e.unlock.invite_key {
            assert_in_all_languages(invite);
        }
    }
}

#[test]
fn every_module_type_has_modtype_name() {
    assert!(!blueprints().module_types.is_empty());
    for t in &blueprints().module_types {
        assert_in_all_languages(&format!("modtype-{}", t.id));
    }
}

#[test]
fn every_hint_kind_has_a_text_in_all_languages() {
    for kind in [
        "mining",
        "surface-prospecting",
        "hge",
        "signal-source",
        "data-scan",
        "mission-reward",
        "salvage",
        "broker",
        "site",
    ] {
        assert_in_all_languages(&format!("hint-{kind}"));
    }
}

#[test]
fn dirty_drive_spanish_name_is_official() {
    assert_eq!(
        I18n::new("es").t("bp-engine_dirty", &[]),
        "Modificación de Motor Sucio"
    );
    assert!(!tmc_i18n::pending_keys("es").contains("bp-engine_dirty"));
}

#[test]
fn engineering_alert_keys_exist_in_all_languages() {
    for key in [
        "engineering-alert-at-max",
        "engineering-alert-engineer-unlocked",
        "engineering-alert-engineer-invited",
        "engineering-alert-scarce-obtained",
        "engineering-alert-goal-completable",
        "engineering-alert-goal-completable-with",
    ] {
        assert_in_all_languages(key);
    }
}
