use super::*;
use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use tmc_model::events::parse_line;
use tmc_model::status::Status;

const TOPIC: &str = "exploration.system";

fn t(seconds: i64) -> DateTime<Utc> {
    "2026-10-18T10:00:00Z".parse::<DateTime<Utc>>().unwrap() + Duration::seconds(seconds)
}

struct H {
    m: Exploration,
    state: GameState,
    clock: i64,
}

impl H {
    fn new() -> Self {
        Self {
            m: Exploration::new(),
            state: GameState::default(),
            clock: 0,
        }
    }

    fn send(&mut self, ev: SourceEvent) -> TopicSet {
        match &ev {
            SourceEvent::Journal { ev, .. } => {
                self.state.apply_event(ev);
            }
            SourceEvent::Status(s) => {
                self.state.apply_status(s);
            }
            _ => {}
        }
        let mut ctx = ModuleCtx::new("exploration", false, 1);
        let dirty = self.m.on_event(&ev, &self.state, &mut ctx);
        let (requests, tasks, _) = ctx.take_outputs();
        assert!(requests.is_empty() && tasks.is_empty());
        dirty
    }

    fn journal(&mut self, name: &str, extra: &str) -> TopicSet {
        self.clock += 1;
        let line = format!(
            r#"{{"timestamp":"{}","event":"{name}"{extra}}}"#,
            t(self.clock).to_rfc3339()
        );
        self.send(SourceEvent::Journal {
            ev: parse_line(&line).unwrap(),
            file: "s.log".into(),
            line: 1,
            replaying: false,
        })
    }

    fn jump(&mut self, system: &str, addr: u64) -> TopicSet {
        self.journal(
            "FSDJump",
            &format!(r#","StarSystem":"{system}","SystemAddress":{addr},"StarPos":[0,0,0]"#),
        )
    }

    fn scan(&mut self, extra: &str) -> TopicSet {
        self.journal("Scan", extra)
    }

    fn settings(&mut self, threshold: u64) -> TopicSet {
        self.send(SourceEvent::Settings(ModuleSettings {
            map_value_threshold: threshold,
            ..ModuleSettings::default()
        }))
    }

    fn status_focus(&mut self, focus: u8) {
        let s = Status::parse(&format!(
            r#"{{"timestamp":"2026-10-18T10:00:00Z","event":"Status","Flags":0,"GuiFocus":{focus}}}"#
        ))
        .unwrap();
        self.send(SourceEvent::Status(s));
    }

    fn topic(&self) -> Value {
        self.m.topic_value(TOPIC).expect("topic")
    }

    fn rows(&self) -> Vec<Value> {
        self.topic()["rows"].as_array().unwrap().clone()
    }

    fn row(&self, body_id: u64) -> Value {
        self.rows()
            .into_iter()
            .find(|r| r["body_id"] == body_id)
            .unwrap_or_else(|| panic!("row {body_id}"))
    }
}

/// Campos de un `Scan` de planeta (ya descubierto y mapeado salvo que `extra` diga otra cosa).
fn planet(id: u32, name: &str, class: &str, mass: f64, extra: &str) -> String {
    format!(
        r#","BodyName":"Sys {name}","BodyID":{id},"DistanceFromArrivalLS":100.5,"PlanetClass":"{class}","MassEM":{mass},"Radius":3000000.0,"WasDiscovered":true,"WasMapped":true{extra}"#
    )
}

fn elw(id: u32) -> String {
    planet(id, &format!("{id}"), "Earthlike body", 1.0, "")
}

fn metal_rich(id: u32, extra: &str) -> String {
    planet(id, &format!("{id}"), "Metal rich body", 4.0, extra)
}

fn mapped_of(scan_extra: &str) -> u64 {
    let line = format!(r#"{{"timestamp":"2026-10-18T10:00:00Z","event":"Scan"{scan_extra}}}"#);
    let EventKind::Scan(s) = parse_line(&line).unwrap().kind else {
        panic!()
    };
    crate::exploration::value::body_values(&s, true)
        .mapped
        .unwrap()
}

fn advice(r: &Value) -> Vec<String> {
    let Some(value) = r.get("advice") else {
        return vec!["not_worth".to_owned()];
    };
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a.as_str().unwrap().to_owned())
        .collect()
}

fn marked(row: &Value, bit: u8) -> bool {
    row.get("marks").and_then(Value::as_u64).unwrap_or(0) & u64::from(bit) != 0
}

const BIO2: &str = r#","Signals":[{"Type":"$SAA_SignalType_Biological;","Count":2},{"Type":"$SAA_SignalType_Human;","Count":9}]"#;

#[test]
fn fss_progress_counts_scanned_vs_body_count() {
    let mut h = H::new();
    h.jump("Sys", 1);
    h.journal(
        "FSSDiscoveryScan",
        r#","Progress":0.4,"BodyCount":10,"NonBodyCount":2,"SystemName":"Sys","SystemAddress":1"#,
    );
    for id in 1..=4 {
        h.scan(&planet(id, &id.to_string(), "Icy body", 0.01, ""));
    }
    let fss = h.topic()["fss"].clone();
    assert_eq!(fss["body_count"], 10);
    assert_eq!(fss["scanned"], 4);
    assert_eq!(fss["complete"], false);
    assert_eq!(fss["progress"], 0.4);
    assert_eq!(h.topic()["unidentified"], 6);
    assert_eq!(h.topic()["system"], "Sys");
    assert_eq!(h.topic()["system_address"], 1);
}

#[test]
fn all_bodies_found_marks_complete() {
    let mut h = H::new();
    h.jump("Sys", 1);
    h.journal(
        "FSSDiscoveryScan",
        r#","Progress":1.0,"BodyCount":10,"NonBodyCount":0,"SystemAddress":1"#,
    );
    assert_eq!(h.topic()["fss"]["complete"], false);
    h.journal(
        "FSSAllBodiesFound",
        r#","SystemName":"Sys","SystemAddress":1,"Count":10"#,
    );
    assert_eq!(h.topic()["fss"]["complete"], true);
    assert_eq!(h.topic()["fss"]["body_count"], 10);
}

#[test]
fn rows_sorted_by_mapped_value_desc() {
    let mut h = H::new();
    h.jump("Sys", 1);
    h.scan(&metal_rich(2, ""));
    h.scan(
        r#","BodyName":"Sys A","BodyID":1,"DistanceFromArrivalLS":0.0,"StarType":"K","StellarMass":0.9,"WasDiscovered":true"#,
    );
    h.scan(&elw(3));
    let rows = h.rows();
    let ids: Vec<u64> = rows
        .iter()
        .map(|r| r["body_id"].as_u64().unwrap())
        .collect();
    assert_eq!(ids, vec![3, 2, 1]);
    let elw_mapped = rows[0]["mapped_value"].as_u64().unwrap();
    assert!((1_500_000..1_580_000).contains(&elw_mapped), "{elw_mapped}");
    assert_eq!(rows[0]["scanned_value"], 283_629);
    let mr = rows[1]["mapped_value"].as_u64().unwrap();
    assert!((190_000..225_000).contains(&mr), "{mr}");
    // Estrella: sin valor de mapeo (campo omitido) y valor escaneado de K (0,9 Msol).
    assert!(rows[2].get("mapped_value").is_none());
    assert_eq!(rows[2]["scanned_value"], 1_216);
}

#[test]
fn advice_map_over_threshold_and_land_with_bio() {
    let mut h = H::new();
    h.jump("Sys", 1);
    let land = ",\"Landable\":true";
    h.scan(&elw(1)); // 1,5 M sin bio
    h.scan(&metal_rich(2, land)); // 200 k con bio 2
    h.scan(&format!("{}{}", elw(3), land)); // 1,5 M con bio 2
    h.scan(&metal_rich(4, "")); // 200 k sin bio
    for id in [2, 3] {
        h.journal(
            "FSSBodySignals",
            &format!(r#","BodyName":"Sys {id}","BodyID":{id},"SystemAddress":1{BIO2}"#),
        );
    }
    assert_eq!(advice(&h.row(1)), vec!["map"]);
    assert_eq!(advice(&h.row(2)), vec!["land"]);
    assert_eq!(advice(&h.row(3)), vec!["map", "land"]);
    assert_eq!(advice(&h.row(4)), vec!["not_worth"]);
    assert_eq!(h.row(3)["bio"], 2);
    assert!(h.row(1).get("bio").is_none());
}

#[test]
fn threshold_change_via_settings_recomputes_advice() {
    let mut h = H::new();
    h.jump("Sys", 1);
    let extra = ",\"TerraformState\":\"Terraformable\"";
    let body = planet(1, "1", "Metal rich body", 1.0, extra);
    let v = mapped_of(&body);
    assert!((700_000..800_000).contains(&v), "{v}");
    h.scan(&body);
    assert_eq!(h.row(1)["mapped_value"], v);
    assert_eq!(advice(&h.row(1)), vec!["map"]);
    let dirty = h.settings(1_000_000);
    assert!(dirty.contains(TOPIC));
    assert_eq!(advice(&h.row(1)), vec!["not_worth"]);
    // Mismo umbral otra vez: nada cambia.
    assert!(h.settings(1_000_000).is_empty());
}

#[test]
fn saa_complete_replaces_estimated_probes() {
    let mut h = H::new();
    h.jump("Sys", 1);
    h.scan(&elw(5));
    let before = h.row(5);
    assert_eq!(before["probes"]["estimated"], true);
    assert_eq!(before["probes"]["n"], 5);
    assert!(!marked(&before, 32));
    let est = before["mapped_value"].as_u64().unwrap();

    h.journal(
        "SAAScanComplete",
        r#","BodyName":"Sys 5","BodyID":5,"ProbesUsed":6,"EfficiencyTarget":6"#,
    );
    let r = h.row(5);
    assert_eq!(r["probes"], json!({"n": 6}));
    assert!(marked(&r, 32));
    assert_eq!(r["mapped_value"], est); // dentro del objetivo: eficiencia conservada
    assert_eq!(advice(&r), vec!["not_worth"]); // ya mapeado por ti: no se aconseja mapear

    // Por encima del objetivo se pierde el bonus de eficiencia (x1,25).
    h.journal(
        "SAAScanComplete",
        r#","BodyName":"Sys 5","BodyID":5,"ProbesUsed":9,"EfficiencyTarget":6"#,
    );
    let worse = h.row(5)["mapped_value"].as_u64().unwrap();
    assert!(
        (worse as f64 - est as f64 / 1.25).abs() <= 2.0,
        "{worse} {est}"
    );
    assert_eq!(h.row(5)["probes"], json!({"n": 6}));
}

#[test]
fn duplicate_scan_updates_row_not_duplicates() {
    let mut h = H::new();
    h.jump("Sys", 1);
    h.scan(&planet(7, "7", "Icy body", 0.01, ""));
    let first = h.row(7)["scanned_value"].as_u64().unwrap();
    h.scan(&planet(7, "7", "Earthlike body", 1.0, ""));
    assert_eq!(h.rows().len(), 1);
    assert_eq!(h.row(7)["scanned_value"], 283_629);
    assert_eq!(h.row(7)["kind_key"], "earthlike");
    assert_ne!(first, 283_629);
    assert_eq!(h.topic()["fss"]["scanned"], 1);
}

#[test]
fn jump_resets_system() {
    let mut h = H::new();
    h.jump("Sys", 1);
    h.scan(&elw(1));
    h.journal("FSSDiscoveryScan", r#","BodyCount":3,"SystemAddress":1"#);
    assert_eq!(h.topic()["fss"]["scanned"], 1);
    h.jump("Other", 2);
    let topic = h.topic();
    assert_eq!(topic["system"], "Other");
    assert!(topic["rows"].as_array().unwrap().is_empty());
    assert_eq!(topic["fss"]["scanned"], 0);
    assert!(topic["fss"].get("body_count").is_none());
    assert_eq!(topic["value_now"], 0);
    assert_eq!(topic["value_if_mapped"], 0);
    assert!(topic.get("unidentified").is_none());
    // Location del mismo sistema no reinicia.
    h.scan(&elw(1));
    h.journal(
        "Location",
        r#","StarSystem":"Other","SystemAddress":2,"StarPos":[0,0,0]"#,
    );
    assert_eq!(h.rows().len(), 1);
    h.journal(
        "Location",
        r#","StarSystem":"Third","SystemAddress":3,"StarPos":[0,0,0]"#,
    );
    assert!(h.rows().is_empty());
}

#[test]
fn body_row_scoopable_uses_shared_star_flags() {
    let mut h = H::new();
    h.jump("Sys", 1);
    let star = |id: u32, class: &str| {
        format!(
            r#","BodyName":"Sys {id}","BodyID":{id},"DistanceFromArrivalLS":0.0,"StarType":"{class}","StellarMass":1.0"#
        )
    };
    h.scan(&star(1, "K"));
    h.scan(&star(2, "N"));
    assert!(marked(&h.row(1), 4));
    assert!(!marked(&h.row(2), 4));
    assert_eq!(h.row(1)["kind_key"], "star-main");
    assert_eq!(h.row(2)["kind_key"], "star-neutron");
}

#[test]
fn relevance_active_in_fss_or_with_unscanned_bodies() {
    let mut h = H::new();
    h.jump("Sys", 1);
    assert_eq!(h.m.relevance(&h.state), Relevance::Normal);
    h.journal("FSSDiscoveryScan", r#","BodyCount":10,"SystemAddress":1"#);
    for id in 1..=4 {
        h.scan(&planet(id, &id.to_string(), "Icy body", 0.01, ""));
    }
    assert_eq!(h.m.relevance(&h.state), Relevance::Active);
    for id in 5..=10 {
        h.scan(&planet(id, &id.to_string(), "Icy body", 0.01, ""));
    }
    h.status_focus(0);
    assert_eq!(h.m.relevance(&h.state), Relevance::Normal);
    h.status_focus(9);
    assert_eq!(h.m.relevance(&h.state), Relevance::Active);
}

#[test]
fn discovered_badge_and_first_discovery_from_was_discovered() {
    let mut h = H::new();
    h.jump("Sys", 1);
    assert!(h.topic().get("discovered").is_none());
    h.scan(
        r#","BodyName":"Sys","BodyID":0,"DistanceFromArrivalLS":0.0,"StarType":"G","StellarMass":1.0,"WasDiscovered":false"#,
    );
    assert_eq!(h.topic()["discovered"], false);
    assert!(marked(&h.row(0), 8));
    assert_eq!(h.row(0)["short_name"], "Sys");
    // Estrella principal con valor x2,6.
    assert_eq!(h.row(0)["scanned_value"], 3_167);
    h.scan(&elw(2));
    assert!(!marked(&h.row(2), 8));
    assert!(marked(&h.row(2), 16));
    assert_eq!(h.row(2)["short_name"], "2");
}

#[test]
fn signals_saa_overrides_fss_and_rings_are_not_rows() {
    let mut h = H::new();
    h.jump("Sys", 1);
    h.scan(&metal_rich(2, ",\"Landable\":true"));
    h.scan(r#","BodyName":"Sys 2 A Ring","BodyID":9,"DistanceFromArrivalLS":100.0"#);
    assert_eq!(h.rows().len(), 1);
    assert_eq!(h.topic()["fss"]["scanned"], 1);
    h.journal(
        "FSSBodySignals",
        r#","BodyName":"Sys 2","BodyID":2,"Signals":[{"Type":"$SAA_SignalType_Biological;","Count":2},{"Type":"$SAA_SignalType_Geological;","Count":1}]"#,
    );
    assert_eq!(
        (h.row(2)["bio"].clone(), h.row(2)["geo"].clone()),
        (json!(2), json!(1))
    );
    h.journal(
        "SAASignalsFound",
        r#","BodyName":"Sys 2","BodyID":2,"Signals":[{"Type":"$SAA_SignalType_Biological;","Count":3}],"Genuses":[]"#,
    );
    assert_eq!(h.row(2)["bio"], 3);
    assert!(h.row(2).get("geo").is_none());
    // Un FSS posterior no pisa al DSS.
    h.journal(
        "FSSBodySignals",
        r#","BodyName":"Sys 2","BodyID":2,"Signals":[{"Type":"$SAA_SignalType_Biological;","Count":1}]"#,
    );
    assert_eq!(h.row(2)["bio"], 3);
}

#[test]
fn values_now_and_if_mapped_follow_definitions() {
    let mut h = H::new();
    h.jump("Sys", 1);
    h.scan(&elw(1));
    h.scan(&metal_rich(2, ""));
    let rows = h.rows();
    let m1 = rows[0]["mapped_value"].as_u64().unwrap();
    let m2 = rows[1]["mapped_value"].as_u64().unwrap();
    let s1 = 283_629u64;
    let s2 = rows[1]["scanned_value"].as_u64().unwrap();
    assert_eq!(h.topic()["value_now"], s1 + s2);
    // Solo el ELW supera el umbral de 500 k.
    assert_eq!(h.topic()["value_if_mapped"], s2 + m1);
    h.journal(
        "SAAScanComplete",
        r#","BodyName":"Sys 1","BodyID":1,"ProbesUsed":5,"EfficiencyTarget":5"#,
    );
    assert_eq!(h.topic()["value_now"], m1 + s2);
    assert_eq!(h.topic()["value_if_mapped"], m1 + s2);
    assert!(m2 < 500_000);
}

#[test]
fn system_with_300_bodies_keeps_topic_under_64_kib() {
    assert_crowded_system_size(&crowded_system(false, false));
}

fn crowded_system(mapped: bool, previously_mapped: bool) -> H {
    let mut h = H::new();
    h.jump("Synuefe XR-H", 1);
    h.journal("FSSDiscoveryScan", r#","BodyCount":320,"SystemAddress":1"#);
    let classes = [
        "Rocky body",
        "Icy body",
        "Metal rich body",
        "High metal content body",
        "Water world",
        "Sudarsky class II gas giant",
        "Rocky ice body",
        "Earthlike body",
    ];
    let mut seed = 12_345u64;
    let mut next = || {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        (seed >> 33) as f64 / (1u64 << 31) as f64
    };
    for id in 1..=300u32 {
        let class = classes[(id as usize) % classes.len()];
        let mass = 0.01 + next() * 20.0;
        let dist = next() * 90_000.0;
        let radius = 1_000_000.0 + next() * 9_000_000.0;
        h.scan(&format!(
            r#","BodyName":"Synuefe XR-H {} {} {}","BodyID":{id},"DistanceFromArrivalLS":{dist},"PlanetClass":"{class}","MassEM":{mass},"Radius":{radius},"Landable":{},"WasDiscovered":{},"WasMapped":{previously_mapped},"TerraformState":"{}""#,
            id % 40,
            (b'a' + (id % 26) as u8) as char,
            id % 5,
            id % 2 == 0,
            previously_mapped || id % 4 != 0,
            if id % 3 == 0 { "Terraformable" } else { "" },
        ));
        if mapped {
            h.journal(
                "SAAScanComplete",
                &format!(r#", "BodyID":{id},"ProbesUsed":6,"EfficiencyTarget":6"#),
            );
            if id % 2 == 0
                && matches!(
                    class,
                    "Rocky body" | "Rocky ice body" | "Icy body" | "High metal content body"
                )
            {
                h.journal("SAASignalsFound",&format!(r#", "BodyID":{id},"Signals":[{{"Type":"$SAA_SignalType_Biological;","Count":4}},{{"Type":"$SAA_SignalType_Geological;","Count":3}}]"#));
            }
        }
    }
    h
}

#[test]
fn system_with_300_mapped_bodies_and_signals_keeps_topic_under_64_kib() {
    let h = crowded_system(true, false);
    let rows = h.rows();
    assert!(rows.iter().all(|r| marked(r, 32)));
    assert!(rows.iter().any(|r| r["bio"] == 4 && r["geo"] == 3));
    assert!(rows.iter().any(|r| marked(r, 1)));
    assert_crowded_system_size(&h);
}

#[test]
fn system_with_300_previously_mapped_bodies_preserves_all_marks_under_64_kib() {
    let h = crowded_system(true, true);
    let rows = h.rows();
    assert!(rows.iter().all(|r| marked(r, 16) && marked(r, 32)));
    assert!(rows.iter().all(|r| !marked(r, 8)));
    assert!(rows.iter().any(|r| r["bio"] == 4 && r["geo"] == 3));
    assert_crowded_system_size(&h);
}

#[test]
fn system_with_300_landable_bodies_all_mapped_and_signalled_stays_under_64_kib() {
    let mut h = H::new();
    h.jump("Sys", 1);
    h.journal("FSSDiscoveryScan", r#", "BodyCount":300,"SystemAddress":1"#);
    for id in 1..=300 {
        h.scan(&planet(
            id,
            &format!("{id} a"),
            "Rocky ice body",
            1.0,
            r#", "Landable":true"#,
        ));
        h.journal(
            "SAAScanComplete",
            &format!(r#", "BodyID":{id},"ProbesUsed":6,"EfficiencyTarget":6"#),
        );
        h.journal("SAASignalsFound",&format!(r#", "BodyID":{id},"Signals":[{{"Type":"$SAA_SignalType_Biological;","Count":4}},{{"Type":"$SAA_SignalType_Geological;","Count":3}}]"#));
    }
    let rows = h.rows();
    assert!(
        rows.iter()
            .all(|r| marked(r, 2) && marked(r, 16) && marked(r, 32))
    );
    assert!(rows.iter().all(|r| r["bio"] == 4 && r["geo"] == 3));
    assert!(rows.iter().all(|r| advice(r) == ["land"]));
    assert_crowded_system_size(&h);
}

fn assert_crowded_system_size(h: &H) {
    let topic = h.topic();
    let bytes = serde_json::to_vec(&topic).unwrap().len();
    println!("topic 300 bodies: {bytes} bytes; row: {}", rows_dbg(&topic));
    assert!(bytes < 65_536, "{bytes}");
    let rows = topic["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 300);
    assert_eq!(
        rows.iter()
            .map(|r| r["body_id"].as_u64().unwrap())
            .collect::<std::collections::BTreeSet<_>>(),
        (1..=300).collect()
    );
    assert_eq!(topic["fss"]["scanned"], 300);
    assert_eq!(
        topic["unidentified"],
        topic["fss"]["body_count"]
            .as_u64()
            .unwrap()
            .saturating_sub(300)
    );
    let mapped: Vec<u64> = rows
        .iter()
        .map(|r| r["mapped_value"].as_u64().unwrap())
        .collect();
    assert!(mapped.windows(2).all(|w| w[0] >= w[1]));
}

#[test]
fn body_row_wire_marks_preserve_all_64_combinations_and_other_fields() {
    use super::system::{Advice, BodyRow, ProbeHint};
    for marks in 0..64u8 {
        let row = BodyRow {
            body_id: 7,
            short_name: "7 a".into(),
            kind_key: "rocky-ice".into(),
            star_class: None,
            distance_ls: Some(100.5),
            scanned_value: 500,
            mapped_value: Some(2708),
            bio: 4,
            geo: 3,
            terraformable: marks & 1 != 0,
            landable: marks & 2 != 0,
            scoopable: marks & 4 != 0,
            first_discovery: marks & 8 != 0,
            already_mapped: marks & 16 != 0,
            mapped_by_you: marks & 32 != 0,
            probes: Some(ProbeHint {
                n: 6,
                estimated: false,
            }),
            advice: vec![Advice::Map, Advice::Land],
        };
        let wire = serde_json::to_value(&row).unwrap();
        assert_eq!(
            wire.get("marks").and_then(Value::as_u64).unwrap_or(0),
            u64::from(marks)
        );
        if marks == 0 {
            assert!(wire.get("marks").is_none());
        }
        for (bit, expected) in [
            (1, row.terraformable),
            (2, row.landable),
            (4, row.scoopable),
            (8, row.first_discovery),
            (16, row.already_mapped),
            (32, row.mapped_by_you),
        ] {
            assert_eq!(marked(&wire, bit), expected);
        }
        for key in [
            "terraformable",
            "landable",
            "scoopable",
            "first_discovery",
            "already_mapped",
            "mapped_by_you",
        ] {
            assert!(wire.get(key).is_none());
        }
        assert_eq!(wire["short_name"], "7 a");
        assert_eq!(wire["kind_key"], "rocky-ice");
        assert_eq!(wire["distance_ls"], 100.5);
        assert_eq!(wire["scanned_value"], 500);
        assert_eq!(wire["mapped_value"], 2708);
        assert_eq!(wire["bio"], 4);
        assert_eq!(wire["geo"], 3);
        assert_eq!(wire["probes"], json!({"n":6}));
        assert_eq!(advice(&wire), ["map", "land"]);
    }
}

#[test]
fn terraforming_marker_matches_terraformable_bonus_but_not_terraformed() {
    let mut h = H::new();
    h.jump("Sys", 1);
    for (id, state) in [
        (1, "Terraformable"),
        (2, "Terraforming"),
        (3, "Terraformed"),
    ] {
        h.scan(&planet(
            id,
            &id.to_string(),
            "High metal content body",
            1.0,
            &format!(r#", "TerraformState":"{state}""#),
        ));
    }
    assert!(marked(&h.row(1), 1));
    assert!(marked(&h.row(2), 1));
    assert!(!marked(&h.row(3), 1));
    assert_eq!(h.row(1)["scanned_value"], h.row(2)["scanned_value"]);
    assert!(
        h.row(1)["scanned_value"].as_u64().unwrap() > h.row(3)["scanned_value"].as_u64().unwrap()
    );
}

fn rows_dbg(topic: &Value) -> String {
    topic["rows"][0].to_string()
}
