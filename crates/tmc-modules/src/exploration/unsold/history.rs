use super::{LedgerContext, UnsoldLedger};
use crate::history::{Position, for_each_after, for_each_before};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use tmc_core::{Store, StoreError, StoreTask};
use tmc_model::events::{EventKind, parse_line};

const RESET_NAMES: &[&str] = &["Died", "MultiSellExplorationData"];
const CONTEXT_NAMES: &[&str] = &["LoadGame", "FSDJump", "CarrierJump", "Location"];
const REBUILD_NAMES: &[&str] = &[
    "LoadGame",
    "FSDJump",
    "CarrierJump",
    "Location",
    "Scan",
    "SAAScanComplete",
    "SellExplorationData",
    "MultiSellExplorationData",
    "Died",
];

pub fn rebuild_from_history(cmdr: String) -> StoreTask {
    Box::new(move |store| {
        let rows = store.query_json("SELECT COALESCE(MAX(id), 0) FROM event", &[])?;
        let cut = rows[0][0].as_i64().unwrap_or_default();
        rebuild_at_cut(store, &cmdr, cut)
    })
}

fn rebuild_at_cut(store: &Store, cmdr: &str, cut: i64) -> Result<Value, StoreError> {
    let reset = latest_reset(store, cmdr, cut)?;
    let mut context = context_before_reset(store, cmdr, cut, reset.as_ref())?;
    let mut ledger = UnsoldLedger::default();
    for_each_after(store, cmdr, cut, REBUILD_NAMES, reset, |pos| {
        if let Ok(event) = parse_line(&pos.raw) {
            context.apply(&mut ledger, &event.kind);
        }
    })?;
    let files = store.query_json(
        "SELECT journal_file.name, MAX(event.line) FROM event
         JOIN journal_file ON journal_file.id = event.file_id
         WHERE event.cmdr = ?1 AND event.id <= ?2 GROUP BY journal_file.name",
        &[json!(cmdr), json!(cut)],
    )?;
    let files: BTreeMap<_, _> = files
        .into_iter()
        .filter_map(|row| Some((row[0].as_str()?.to_owned(), row[1].as_u64()?)))
        .collect();
    let ledger =
        serde_json::to_value(ledger).map_err(|error| StoreError::Unsupported(error.to_string()))?;
    let context = serde_json::to_value(context)
        .map_err(|error| StoreError::Unsupported(error.to_string()))?;
    Ok(json!({"cmdr":cmdr,"ledger":ledger,"context":context,"files":files,"cut":cut}))
}

fn latest_reset(store: &Store, cmdr: &str, cut: i64) -> Result<Option<Position>, StoreError> {
    let mut found = None;
    for_each_before(store, cmdr, cut, RESET_NAMES, None, |pos| {
        let Ok(event) = parse_line(&pos.raw) else {
            return false;
        };
        let reset = match event.kind {
            EventKind::Died => true,
            EventKind::MultiSellExplorationData(sale) => sale.discovered.is_empty(),
            _ => false,
        };
        if reset {
            found = Some(pos.clone());
        }
        reset
    })?;
    Ok(found)
}

fn context_before_reset(
    store: &Store,
    cmdr: &str,
    cut: i64,
    reset: Option<&Position>,
) -> Result<LedgerContext, StoreError> {
    let mut context = LedgerContext::default();
    let Some(reset) = reset else {
        return Ok(context);
    };
    let mut ignored_ledger = UnsoldLedger::default();
    let mut found_system = false;
    let mut found_odyssey = false;
    for_each_before(store, cmdr, cut, CONTEXT_NAMES, Some(reset), |pos| {
        if let Ok(event) = parse_line(&pos.raw) {
            match &event.kind {
                EventKind::LoadGame(load) if !found_odyssey && load.odyssey.is_some() => {
                    context.apply(&mut ignored_ledger, &event.kind);
                    found_odyssey = true;
                }
                EventKind::Location(_) | EventKind::FsdJump(_) | EventKind::CarrierJump(_)
                    if !found_system =>
                {
                    context.apply(&mut ignored_ledger, &event.kind);
                    found_system = true;
                }
                _ => {}
            }
        }
        found_system && found_odyssey
    })?;
    Ok(context)
}

#[cfg(test)]
mod tests {
    use super::super::{LedgerContext, UnsoldLedger};
    use super::*;
    use serde_json::{Value, json};
    use tmc_core::{EventWriter, Store, StoredEvent};
    use tmc_model::events::parse_line;

    fn event(name: &str, fields: Value) -> String {
        let mut raw = fields;
        raw["timestamp"] = json!("2026-10-18T10:00:00Z");
        raw["event"] = json!(name);
        raw.to_string()
    }

    fn scan(id: u32) -> String {
        event(
            "Scan",
            json!({"BodyID":id,"BodyName":format!("Synthetic {id}"),"PlanetClass":"Earthlike body","MassEM":1.0,"WasDiscovered":true,"WasMapped":true}),
        )
    }

    fn write_events(store: &Store, file: &str, cmdr: &str, lines: &[String]) {
        write_rows(
            store,
            file,
            cmdr,
            lines
                .iter()
                .enumerate()
                .map(|(index, line)| (index as u64 + 1, line.clone())),
        );
    }

    fn write_rows(
        store: &Store,
        file: &str,
        cmdr: &str,
        lines: impl IntoIterator<Item = (u64, String)>,
    ) {
        let writer = EventWriter::spawn(store);
        for (line_number, line) in lines {
            let parsed = parse_line(&line).unwrap();
            writer.send(StoredEvent {
                file_name: file.into(),
                line: line_number,
                ts: parsed.timestamp,
                name: parsed.name,
                cmdr: Some(cmdr.into()),
                json: line,
            });
        }
        writer.flush_blocking().unwrap();
    }

    fn rebuilt(store: &Store) -> (Value, UnsoldLedger) {
        let result = rebuild_from_history("Synthetic Commander".into())(store).unwrap();
        let ledger = serde_json::from_value(result["ledger"].clone()).unwrap();
        (result, ledger)
    }

    #[test]
    fn rebuild_from_history_matches_live_accumulation() {
        let store = Store::open_in_memory().unwrap();
        let lines: Vec<_> =
            include_str!("../../../../../tests/fixtures/journal/exploration_session.log")
                .lines()
                .take_while(|line| !line.contains("SellExplorationData"))
                .map(str::to_owned)
                .collect();
        write_events(&store, "fixture.log", "Synthetic Commander", &lines);
        let mut live = UnsoldLedger::default();
        let mut context = LedgerContext::default();
        for line in &lines {
            context.apply(&mut live, &parse_line(line).unwrap().kind);
        }
        assert!(live.total() > 0);
        let (_, ledger) = rebuilt(&store);
        assert_eq!(ledger.total(), live.total());
        assert_eq!(ledger.system_count(), live.system_count());
    }

    #[test]
    fn partial_sales_and_empty_single_sale_preserve_other_systems() {
        let store = Store::open_in_memory().unwrap();
        let lines = vec![
            event("Location", json!({"StarSystem":"A"})),
            scan(1),
            event("FSDJump", json!({"StarSystem":"B"})),
            scan(1),
            event("SellExplorationData", json!({"Systems":[]})),
            event(
                "MultiSellExplorationData",
                json!({"Discovered":[{"SystemName":"A","NumBodies":1}]}),
            ),
        ];
        write_events(&store, "partial.log", "Synthetic Commander", &lines);
        let (_, ledger) = rebuilt(&store);
        let mut expected = UnsoldLedger::default();
        expected.apply_event("B", &parse_line(&scan(1)).unwrap().kind, true);
        assert!(expected.total() > 0);
        assert_eq!(ledger.total(), expected.total());
        assert_eq!(ledger.system_count(), 1);
    }

    #[test]
    fn reset_recovers_prior_context_and_keeps_full_file_watermarks() {
        for reset in [
            event("Died", json!({})),
            event("MultiSellExplorationData", json!({"Discovered":[]})),
        ] {
            let store = Store::open_in_memory().unwrap();
            write_events(
                &store,
                "a-before.log",
                "Synthetic Commander",
                &[
                    event("LoadGame", json!({"Odyssey":false})),
                    event("LoadGame", json!({})),
                    event("Location", json!({"StarSystem":"A"})),
                    scan(1),
                    reset,
                ],
            );
            write_events(
                &store,
                "b-after.log",
                "Synthetic Commander",
                &[scan(2), event("Music", json!({}))],
            );
            write_events(
                &store,
                "other.log",
                "Other Commander",
                &[event("Died", json!({}))],
            );
            let (result, ledger) = rebuilt(&store);
            let mut expected = UnsoldLedger::default();
            expected.apply_event("A", &parse_line(&scan(2)).unwrap().kind, false);
            assert!(expected.total() > 0);
            assert_eq!(ledger.total(), expected.total());
            assert_eq!(result["context"]["system"], "A");
            assert_eq!(result["context"]["odyssey"], false);
            assert_eq!(result["files"], json!({"a-before.log":5,"b-after.log":2}));
            assert_eq!(result["cut"], 8);
            assert_eq!(result["cmdr"], "Synthetic Commander");
        }
    }

    #[test]
    fn rebuild_streams_multiple_pages_in_id_order() {
        let store = Store::open_in_memory().unwrap();
        let mut lines = vec![event("Location", json!({"StarSystem":"A"}))];
        lines.extend((1..=600).map(scan));
        write_events(&store, "many.log", "Synthetic Commander", &lines);
        let mut expected = UnsoldLedger::default();
        for line in lines.iter().skip(1) {
            expected.apply_event("A", &parse_line(line).unwrap().kind, true);
        }
        let (result, ledger) = rebuilt(&store);
        assert_eq!(ledger.total(), expected.total());
        assert_eq!(ledger.system_count(), 1);
        assert_eq!(result["files"]["many.log"], 601);
    }

    #[test]
    fn stable_cut_excludes_later_resets_context_and_file_lines() {
        let store = Store::open_in_memory().unwrap();
        write_events(
            &store,
            "cut.log",
            "Synthetic Commander",
            &[
                event("LoadGame", json!({"Odyssey":false})),
                event("Location", json!({"StarSystem":"A"})),
                scan(1),
                event("Died", json!({})),
                event("LoadGame", json!({"Odyssey":true})),
                event("Location", json!({"StarSystem":"B"})),
                scan(2),
            ],
        );
        let result = rebuild_at_cut(&store, "Synthetic Commander", 3).unwrap();
        let ledger: UnsoldLedger = serde_json::from_value(result["ledger"].clone()).unwrap();
        let mut expected = UnsoldLedger::default();
        expected.apply_event("A", &parse_line(&scan(1)).unwrap().kind, false);
        assert_eq!(ledger.total(), expected.total());
        assert!(ledger.total() > 0);
        assert_eq!(result["context"]["system"], "A");
        assert_eq!(result["context"]["odyssey"], false);
        assert_eq!(result["files"]["cut.log"], 3);
        assert_eq!(result["cut"], 3);
    }

    #[test]
    fn old_death_imported_after_live_scan_does_not_erase_new_data() {
        let store = Store::open_in_memory().unwrap();
        let at = |line: String, timestamp: &str| {
            let mut raw: Value = serde_json::from_str(&line).unwrap();
            raw["timestamp"] = json!(timestamp);
            raw.to_string()
        };
        write_events(
            &store,
            "live.log",
            "Synthetic Commander",
            &[
                at(
                    event("Location", json!({"StarSystem":"A"})),
                    "2026-10-18T12:00:00Z",
                ),
                at(scan(1), "2026-10-18T12:00:01Z"),
            ],
        );
        write_events(
            &store,
            "old.log",
            "Synthetic Commander",
            &[
                event("LoadGame", json!({"Odyssey":false})),
                event("Location", json!({"StarSystem":"A"})),
                event("Died", json!({})),
            ],
        );
        let (result, ledger) = rebuilt(&store);
        let mut expected = UnsoldLedger::default();
        expected.apply_event("A", &parse_line(&scan(1)).unwrap().kind, false);
        assert!(expected.total() > 0);
        assert_eq!(ledger.total(), expected.total());
        assert_eq!(result["context"]["system"], "A");
        assert_eq!(result["context"]["odyssey"], false);
    }

    #[test]
    fn same_second_events_follow_journal_lines_not_insertion_ids() {
        for action in ["map", "sell", "die"] {
            let store = Store::open_in_memory().unwrap();
            let mut lines = vec![event("Location", json!({"StarSystem":"A"})), scan(1)];
            if action == "sell" {
                lines.push(event("SellExplorationData", json!({"Systems":["A"]})));
            } else if action == "die" {
                lines.push(event("Died", json!({})));
                lines.push(scan(1));
            }
            lines.push(event(
                "SAAScanComplete",
                json!({"BodyID":1,"ProbesUsed":6,"EfficiencyTarget":6}),
            ));
            let mut live = UnsoldLedger::default();
            let mut context = LedgerContext::default();
            for line in &lines {
                context.apply(&mut live, &parse_line(line).unwrap().kind);
            }
            let reversed = lines
                .into_iter()
                .enumerate()
                .rev()
                .map(|(index, line)| (index as u64 + 1, line));
            write_rows(&store, "same-second.log", "Synthetic Commander", reversed);
            let (_, ledger) = rebuilt(&store);
            assert_eq!(ledger.total(), live.total(), "action={action}");
            if action == "sell" {
                assert_eq!(ledger.total(), 0);
            } else {
                assert!(ledger.total() > 0);
            }
        }
    }
}
