//! Decisiones internas de los módulos: reproducibles, sin E/S y sin datos privados.
use serde_json::json;
use tmc_core::ModuleCtx;
use tmc_model::{
    diagnostics::capture::capture, events::parse_line, source::SourceEvent, state::GameState,
};

fn journal(name: &str, extra: &str) -> SourceEvent {
    SourceEvent::Journal {
        ev: parse_line(&format!(
            r#"{{"timestamp":"2026-10-17T18:00:00Z","event":"{name}"{extra}}}"#
        ))
        .unwrap(),
        file: "Journal.synthetic.log".into(),
        line: 1,
        replaying: false,
    }
}

fn action(name: &str) -> SourceEvent {
    SourceEvent::Action {
        name: name.into(),
        args: json!({}),
    }
}

fn run() -> Vec<(String, String)> {
    let mut state = GameState::default();
    let load = journal(
        "LoadGame",
        r#","Commander":"CANARY-CMDR","FID":"F-CANARY-FID","Ship":"python","ShipName":"CANARY-SHIP","ShipIdent":"CANARY-ID""#,
    );
    if let SourceEvent::Journal { ev, .. } = &load {
        state.apply_event(ev);
    }
    let events = [
        load,
        action("combat.reset_session"),
        action("mining.reset_session"),
        journal(
            "ShipTargeted",
            r#","TargetLocked":true,"Ship":"anaconda","ScanStage":3"#,
        ),
        journal(
            "ProspectedAsteroid",
            r#","Materials":[{"Name":"Platinum","Proportion":20.0}],"Content":"$AsteroidMaterialContent_High;","Remaining":100.0"#,
        ),
        journal("Shutdown", ""),
    ];
    let mut modules = crate::all_modules();
    let ((), records) = capture(|| {
        for event in &events {
            for module in &mut modules {
                let mut ctx = ModuleCtx::new(module.id(), false, 1);
                module.on_event(event, &state, &mut ctx);
            }
        }
    });
    records.into_iter().map(|r| (r.code, r.payload)).collect()
}

#[test]
fn module_decisions_are_reproducible_and_private() {
    let first = run();
    let codes: Vec<&str> = first.iter().map(|(c, _)| c.as_str()).collect();
    for expected in [
        "combat.session.reset.applied",
        "mining.session.reset.applied",
        "dashboard.session.reset",
        "combat.target.changed",
        "mining.prospector.verdict",
    ] {
        assert!(codes.contains(&expected), "{expected} missing in {codes:?}");
    }
    assert!(first.iter().all(|(_, p)| !p.contains("CANARY")));
    assert_eq!(first, run());
}
