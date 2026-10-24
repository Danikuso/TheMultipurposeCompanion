//! Ingenieros: estado, rango, distancia y siguiente paso de desbloqueo.

use std::collections::HashMap;

use serde::Serialize;
use tmc_model::events::{EngineerEntry, EngineerProgress, EngineerStatus, EventKind};

use crate::data::{EngineerDef, engineer, engineers};

/// Cambio de estado de un ingeniero entre dos `EngineerProgress`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EngineerStatusChange {
    pub id: u64,
    pub from: Option<EngineerStatus>,
    pub to: EngineerStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Key {
    Id(u64),
    /// Entrada sin `EngineerID` cuyo nombre no está catalogado (minúsculas).
    Name(String),
}

fn engineer_name_matches(catalog: &str, candidate: &str) -> bool {
    let cat = catalog.trim().to_lowercase();
    let cand = candidate.trim().to_lowercase();
    if cat == cand {
        return true;
    }
    // "Tod 'The Blaster' McQuinn" en catálogo vs "Tod McQuinn" en el diario de Frontier
    if cat.contains('\'') {
        let stripped: String = cat
            .split('\'')
            .enumerate()
            .filter(|(i, _)| i % 2 == 0)
            .map(|(_, part)| part)
            .collect::<Vec<_>>()
            .join(" ");
        let cleaned = stripped.split_whitespace().collect::<Vec<_>>().join(" ");
        if cleaned == cand {
            return true;
        }
    }
    false
}

fn key_of(e: &EngineerEntry) -> Key {
    if e.engineer_id != 0 {
        return Key::Id(e.engineer_id);
    }
    let lower = e.engineer.trim().to_lowercase();
    engineers()
        .iter()
        .find(|d| engineer_name_matches(&d.name, &lower))
        .map_or(Key::Name(lower), |d| Key::Id(d.id))
}

/// `true` si `ID` y nombre se contradicen: el ID catalogado tiene otro nombre, o el
/// nombre catalogado tiene otro ID. Una entrada así no se aplica (no pisa a nadie).
fn contradicts(e: &EngineerEntry) -> bool {
    let name = e.engineer.trim().to_lowercase();
    if e.engineer_id == 0 || name.is_empty() {
        return false;
    }
    if let Some(d) = engineer(e.engineer_id) {
        return !engineer_name_matches(&d.name, &name);
    }
    engineers()
        .iter()
        .any(|d| engineer_name_matches(&d.name, &name))
}

/// `true` si el JSON trae `StarPos` con tres números.
fn has_star_pos(raw: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|v| v.get("StarPos").and_then(|p| p.as_array().cloned()))
        .is_some_and(|a| a.len() == 3 && a.iter().all(|c| c.is_number()))
}

/// Estado conocido de los ingenieros y posición del comandante.
#[derive(Debug, Clone, Default)]
pub struct EngineerBook {
    entries: HashMap<Key, EngineerEntry>,
    /// `None` hasta el primer `Location`/`FSDJump`/`CarrierJump`.
    position: Option<[f64; 3]>,
}

impl EngineerBook {
    pub fn position(&self) -> Option<[f64; 3]> {
        self.position
    }

    pub fn status(&self, id: u64) -> Option<&EngineerEntry> {
        self.entries.get(&Key::Id(id))
    }

    /// La lista de arranque sustituye el estado sin generar cambios; la
    /// actualización suelta fusiona y devuelve el cambio de estado, si lo hay.
    /// Un `Engineers: []` vacío no borra nada.
    pub fn apply(&mut self, p: &EngineerProgress) -> Vec<EngineerStatusChange> {
        let list = p.entries();
        if p.is_startup_list() {
            if !list.is_empty() {
                self.entries = list
                    .into_iter()
                    .filter(|e| !contradicts(e))
                    .map(|mut e| {
                        let key = key_of(&e);
                        if let Key::Id(id) = key {
                            e.engineer_id = id;
                        }
                        (key, e)
                    })
                    .collect();
            }
            return Vec::new();
        }
        let mut changes = Vec::new();
        for mut entry in list {
            if contradicts(&entry) {
                continue;
            }
            let key = key_of(&entry);
            if let Key::Id(id) = key {
                entry.engineer_id = id;
            }
            let old = self.entries.get(&key);
            if let Some(old) = old {
                // Una actualización sin `Progress`/`Rank` conserva lo anterior.
                if entry.progress == EngineerStatus::Unknown {
                    entry.progress = old.progress;
                }
                entry.rank = entry.rank.or(old.rank);
                entry.rank_progress = entry.rank_progress.or(old.rank_progress);
                if entry.engineer.is_empty() {
                    entry.engineer = old.engineer.clone();
                }
            }
            let from = old.map(|o| o.progress);
            if from != Some(entry.progress) {
                changes.push(EngineerStatusChange {
                    id: entry.engineer_id,
                    from,
                    to: entry.progress,
                });
            }
            self.entries.insert(key, entry);
        }
        changes
    }

    /// Solo `EngineerProgress`: la posición exige el JSON original (ver [`Self::apply_journal`]).
    pub fn apply_event(&mut self, kind: &EventKind) -> Vec<EngineerStatusChange> {
        match kind {
            EventKind::EngineerProgress(p) => self.apply(p),
            _ => Vec::new(),
        }
    }

    /// Como `apply_event`, pero también toma la posición de `Location`/`FSDJump`/
    /// `CarrierJump` si el JSON original trae `StarPos`: un evento sin él no fija la
    /// posición (el valor por defecto sería Sol).
    pub fn apply_journal(&mut self, kind: &EventKind, raw: &str) -> Vec<EngineerStatusChange> {
        match kind {
            EventKind::Location(l) | EventKind::FsdJump(l) | EventKind::CarrierJump(l) => {
                if has_star_pos(raw) && l.star_pos.iter().all(|c| c.is_finite()) {
                    self.position = Some(l.star_pos);
                }
                Vec::new()
            }
            other => self.apply_event(other),
        }
    }

    pub fn rows(&self) -> Vec<EngineerRow> {
        engineer_rows(self, self.position)
    }
}

/// Fila de ingeniero lista para mostrar.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EngineerRow {
    pub id: u64,
    pub name: String,
    pub system: String,
    pub distance_ly: Option<f64>,
    /// `Unknown` si el diario no lo menciona.
    pub status: EngineerStatus,
    pub rank: Option<u8>,
    pub rank_progress: Option<u8>,
    pub on_foot: bool,
    pub verified: bool,
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// Nave antes que a pie; luego por distancia (sin posición, por nombre).
pub fn engineer_rows(book: &EngineerBook, pos: Option<[f64; 3]>) -> Vec<EngineerRow> {
    engineer_rows_in(engineers(), book, pos)
}

/// Como [`engineer_rows`] sobre un catálogo dado (el activo o el de pruebas).
pub(crate) fn engineer_rows_in(
    defs: &[EngineerDef],
    book: &EngineerBook,
    pos: Option<[f64; 3]>,
) -> Vec<EngineerRow> {
    let mut rows: Vec<EngineerRow> = defs
        .iter()
        .map(|d| {
            let entry = book.status(d.id);
            EngineerRow {
                id: d.id,
                name: d.name.clone(),
                system: d.system.clone(),
                distance_ly: pos.map(|p| distance(p, d.star_pos)),
                status: entry.map_or(EngineerStatus::Unknown, |e| e.progress),
                rank: entry.and_then(|e| e.rank),
                rank_progress: entry.and_then(|e| e.rank_progress),
                on_foot: d.on_foot,
                verified: d.verified,
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        a.on_foot
            .cmp(&b.on_foot)
            .then_with(|| match (a.distance_ly, b.distance_ly) {
                (Some(x), Some(y)) => x.total_cmp(&y),
                _ => std::cmp::Ordering::Equal,
            })
            .then_with(|| a.name.cmp(&b.name))
    });
    rows
}

/// Siguiente paso para desbloquear a un ingeniero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum NextStep {
    Done,
    /// Clave Fluent del requisito de invitación.
    Requirement {
        key: String,
    },
    /// Invitado: clave Fluent de qué hacer para desbloquearlo.
    Invite {
        key: String,
    },
    Contribute {
        item: String,
        kind: String,
        have: u32,
        need: u32,
    },
}

/// `Known`/desconocido/vetado piden el requisito; `Invited`/`Acquainted` piden la
/// contribución (válida) o, si no la hay, la clave de invitación; `Unlocked` está hecho.
/// Una contribución con tipo desconocido o cantidad 0 se trata como inexistente.
/// Los créditos no se pueden contar desde el inventario: `have` es 0 (usa
/// [`NextStep::with_credits`] con el saldo del comandante).
pub fn next_step(
    def: &EngineerDef,
    entry: Option<&EngineerEntry>,
    have_material: &dyn Fn(&str) -> u32,
    have_commodity: &dyn Fn(&str) -> u32,
) -> NextStep {
    let status = entry.map_or(EngineerStatus::Unknown, |e| e.progress);
    let requirement = || NextStep::Requirement {
        key: def.unlock.requirement_key.clone(),
    };
    match status {
        EngineerStatus::Unlocked => NextStep::Done,
        EngineerStatus::Invited | EngineerStatus::Acquainted => {
            if let Some(c) = def.unlock.contribution.as_ref().filter(|c| c.is_valid()) {
                let kind = c.kind.to_ascii_lowercase();
                let have = match kind.as_str() {
                    "material" => have_material(&c.item),
                    "commodity" => have_commodity(&c.item),
                    _ => 0,
                };
                return NextStep::Contribute {
                    item: c.item.clone(),
                    kind,
                    have,
                    need: c.amount,
                };
            }
            def.unlock
                .invite_key
                .as_ref()
                .map_or_else(requirement, |key| NextStep::Invite { key: key.clone() })
        }
        _ => requirement(),
    }
}

impl NextStep {
    /// Una contribución en créditos usa el saldo del comandante como `have`
    /// (satura en `u32::MAX`; un saldo desconocido o negativo cuenta 0).
    pub fn with_credits(self, balance: Option<i64>) -> NextStep {
        match self {
            NextStep::Contribute {
                item,
                kind,
                need,
                have,
            } => {
                let have = if kind == "credits" {
                    u32::try_from(balance.unwrap_or(0).max(0)).unwrap_or(u32::MAX)
                } else {
                    have
                };
                NextStep::Contribute {
                    item,
                    kind,
                    have,
                    need,
                }
            }
            other => other,
        }
    }
}

/// Paso siguiente por id de ingeniero catalogado.
pub fn next_step_for(
    book: &EngineerBook,
    id: u64,
    have_material: &dyn Fn(&str) -> u32,
    have_commodity: &dyn Fn(&str) -> u32,
) -> Option<NextStep> {
    engineer(id).map(|d| next_step(d, book.status(id), have_material, have_commodity))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Contribution;
    use tmc_model::events::parse_line;

    const FIXTURE: &str =
        include_str!("../../../../tests/fixtures/journal/engineering_session.log");

    fn ev(json: &str) -> EventKind {
        parse_line(json).unwrap().kind
    }

    fn progress(body: &str) -> EventKind {
        ev(&format!(
            r#"{{"timestamp":"2026-10-21T12:00:00Z","event":"EngineerProgress",{body}}}"#
        ))
    }

    fn row(rows: &[EngineerRow], id: u64) -> &EngineerRow {
        rows.iter().find(|r| r.id == id).unwrap()
    }

    fn none(_: &str) -> u32 {
        0
    }

    #[test]
    fn rows_from_fixture_progress() {
        let mut book = EngineerBook::default();
        let mut lines = FIXTURE.lines();
        for l in lines.by_ref() {
            let kind = parse_line(l).unwrap().kind;
            let is_progress = matches!(kind, EventKind::EngineerProgress(_));
            book.apply_event(&kind);
            if is_progress {
                break;
            }
        }
        let rows = book.rows();
        let f = row(&rows, 300100);
        assert_eq!(
            (f.status, f.rank, f.rank_progress),
            (EngineerStatus::Unlocked, Some(5), Some(0))
        );
        let e = row(&rows, 300160);
        assert_eq!((e.status, e.rank), (EngineerStatus::Invited, None));
        assert_eq!(row(&rows, 300260).status, EngineerStatus::Known);
        assert_eq!(row(&rows, 300220).status, EngineerStatus::Unknown);
        for l in lines {
            book.apply_event(&parse_line(l).unwrap().kind);
        }
        assert_eq!(
            book.status(300160).unwrap().progress,
            EngineerStatus::Unlocked
        );
    }

    #[test]
    fn no_position_until_first_location() {
        let mut book = EngineerBook::default();
        book.apply_event(&progress(r#""Engineers":[]"#));
        let rows = book.rows();
        assert!(rows.iter().all(|r| r.distance_ly.is_none()));
        let ship: Vec<&str> = rows
            .iter()
            .filter(|r| !r.on_foot)
            .map(|r| r.name.as_str())
            .collect();
        let mut sorted = ship.clone();
        sorted.sort_unstable();
        assert_eq!(ship, sorted);
        assert!(rows.last().unwrap().on_foot);
        let raw = r#"{"timestamp":"2026-10-21T12:00:02Z","event":"Location","StarSystem":"Sol","StarPos":[0.0,0.0,0.0]}"#;
        book.apply_journal(&ev(raw), raw);
        assert_eq!(book.position(), Some([0.0, 0.0, 0.0]));
        assert!(book.rows().iter().all(|r| r.distance_ly.is_some()));
    }

    #[test]
    fn location_without_star_pos_does_not_set_position() {
        for event in ["Location", "FSDJump", "CarrierJump"] {
            let mut book = EngineerBook::default();
            let raw = format!(
                r#"{{"timestamp":"2026-10-21T12:00:02Z","event":"{event}","StarSystem":"Nowhere","SystemAddress":5}}"#
            );
            book.apply_journal(&ev(&raw), &raw);
            assert_eq!(book.position(), None, "{event}");
            assert!(book.rows().iter().all(|r| r.distance_ly.is_none()));
        }
        // Una posición válida anterior se conserva.
        let mut book = EngineerBook::default();
        let with = r#"{"timestamp":"2026-10-21T12:00:02Z","event":"FSDJump","StarSystem":"A","StarPos":[1,2,3]}"#;
        book.apply_journal(&ev(with), with);
        let without = r#"{"timestamp":"2026-10-21T12:00:03Z","event":"Location","StarSystem":"B"}"#;
        book.apply_journal(&ev(without), without);
        assert_eq!(book.position(), Some([1.0, 2.0, 3.0]));
    }

    #[test]
    fn distance_from_position() {
        let book = EngineerBook::default();
        let d = engineer(300100).unwrap();
        let expect = d.star_pos.iter().map(|c| c * c).sum::<f64>().sqrt();
        let rows = engineer_rows(&book, Some([0.0, 0.0, 0.0]));
        assert!((row(&rows, 300100).distance_ly.unwrap() - expect).abs() < 0.01);
        let ship: Vec<f64> = rows
            .iter()
            .filter(|r| !r.on_foot)
            .map(|r| r.distance_ly.unwrap())
            .collect();
        assert!(ship.windows(2).all(|w| w[0] <= w[1]));
        assert!(rows.last().unwrap().on_foot);
    }

    #[test]
    fn startup_list_creates_no_changes() {
        let mut book = EngineerBook::default();
        assert!(
            book.apply_event(&progress(
                r#""Engineers":[{"Engineer":"Elvira Martuuk","EngineerID":300160,"Progress":"Invited"}]"#
            ))
            .is_empty()
        );
        let loose = r#""Engineer":"Elvira Martuuk","EngineerID":300160,"Progress":"Unlocked""#;
        assert_eq!(
            book.apply_event(&progress(loose)),
            vec![EngineerStatusChange {
                id: 300160,
                from: Some(EngineerStatus::Invited),
                to: EngineerStatus::Unlocked
            }]
        );
        assert!(book.apply_event(&progress(loose)).is_empty());
    }

    #[test]
    fn empty_list_is_noop_and_loose_without_progress_keeps_status() {
        let mut book = EngineerBook::default();
        book.apply_event(&progress(
            r#""Engineers":[{"Engineer":"Felicity Farseer","EngineerID":300100,"Progress":"Unlocked","Rank":3}]"#,
        ));
        assert!(book.apply_event(&progress(r#""Engineers":[]"#)).is_empty());
        assert!(book.status(300100).is_some());
        let ch = book.apply_event(&progress(
            r#""Engineer":"Felicity Farseer","EngineerID":300100,"Rank":4"#,
        ));
        assert!(ch.is_empty());
        let e = book.status(300100).unwrap();
        assert_eq!((e.progress, e.rank), (EngineerStatus::Unlocked, Some(4)));
    }

    #[test]
    fn engineer_id_zero_is_keyed_by_name() {
        let mut book = EngineerBook::default();
        let ch = book.apply_event(&progress(
            r#""Engineer":"felicity farseer","Progress":"Invited""#,
        ));
        assert_eq!(ch.len(), 1);
        assert_eq!(
            book.status(300100).unwrap().progress,
            EngineerStatus::Invited
        );
        book.apply_event(&progress(r#""Engineer":"Nobody","Progress":"Known""#));
        assert_eq!(book.rows().len(), engineers().len());
    }

    #[test]
    fn next_step_contribution_uses_inventory() {
        let mut def = engineer(300160).unwrap().clone();
        def.unlock.contribution = Some(Contribution {
            item: "iron".into(),
            kind: "material".into(),
            amount: 10,
        });
        let entry = EngineerEntry {
            engineer: def.name.clone(),
            engineer_id: def.id,
            progress: EngineerStatus::Invited,
            rank: None,
            rank_progress: None,
        };
        let step = next_step(
            &def,
            Some(&entry),
            &|k| if k == "iron" { 4 } else { 0 },
            &|_| 99,
        );
        assert_eq!(
            step,
            NextStep::Contribute {
                item: "iron".into(),
                kind: "material".into(),
                have: 4,
                need: 10
            }
        );
        def.unlock.contribution = Some(Contribution {
            item: "iron".into(),
            kind: "gold".into(),
            amount: 10,
        });
        assert_eq!(
            next_step(&def, Some(&entry), &none, &none),
            NextStep::Invite {
                key: def.unlock.invite_key.clone().unwrap()
            }
        );
    }

    #[test]
    fn credits_contribution_uses_the_commander_balance() {
        let step = NextStep::Contribute {
            item: "credits".into(),
            kind: "credits".into(),
            have: 0,
            need: 500_000,
        };
        let with = |b| match step.clone().with_credits(b) {
            NextStep::Contribute { have, .. } => have,
            other => panic!("{other:?}"),
        };
        assert_eq!(with(Some(1_234_567)), 1_234_567);
        assert_eq!(with(Some(i64::MAX)), u32::MAX);
        assert_eq!(with(Some(-5)), 0);
        assert_eq!(with(None), 0);
        // Los materiales no se tocan.
        let mat = NextStep::Contribute {
            item: "iron".into(),
            kind: "material".into(),
            have: 4,
            need: 10,
        };
        assert_eq!(mat.clone().with_credits(Some(99)), mat);
        assert_eq!(NextStep::Done.with_credits(Some(1)), NextStep::Done);
    }

    #[test]
    fn next_step_by_status() {
        let def = engineer(300100).unwrap();
        let mk = |p| EngineerEntry {
            engineer: String::new(),
            engineer_id: def.id,
            progress: p,
            rank: None,
            rank_progress: None,
        };
        let req = NextStep::Requirement {
            key: def.unlock.requirement_key.clone(),
        };
        assert_eq!(next_step(def, None, &none, &none), req);
        assert_eq!(
            next_step(def, Some(&mk(EngineerStatus::Known)), &none, &none),
            req
        );
        assert_eq!(
            next_step(def, Some(&mk(EngineerStatus::Unlocked)), &none, &none),
            NextStep::Done
        );
    }

    #[test]
    fn unknown_engineer_in_journal_ignored_without_panic() {
        let mut book = EngineerBook::default();
        book.apply_event(&progress(
            r#""Engineers":[{"Engineer":"Ghost","EngineerID":999,"Progress":"Unlocked"}]"#,
        ));
        book.apply_event(&progress(
            r#""Engineer":"Ghost","EngineerID":999,"Progress":"Barred""#,
        ));
        let rows = book.rows();
        assert!(rows.iter().all(|r| r.id != 999));
        assert_eq!(rows.len(), engineers().len());
        assert!(rows.iter().all(|r| r.status == EngineerStatus::Unknown));
    }

    #[test]
    fn status_by_name_without_id_resolves_to_the_catalogued_engineer() {
        let mut book = EngineerBook::default();
        book.apply_event(&progress(
            r#""Engineers":[{"Engineer":"Elvira Martuuk","Progress":"Unlocked","Rank":2}]"#,
        ));
        let e = book.status(300160).unwrap();
        assert_eq!(
            (e.progress, e.rank, e.engineer_id),
            (EngineerStatus::Unlocked, Some(2), 300160)
        );
    }

    #[test]
    fn id_name_contradiction_never_overwrites_a_known_engineer() {
        let mut book = EngineerBook::default();
        book.apply_event(&progress(
            r#""Engineers":[{"Engineer":"Felicity Farseer","EngineerID":300100,"Progress":"Unlocked","Rank":3}]"#,
        ));
        // ID de Felicity con el nombre de Elvira (y al revés): se ignora.
        for body in [
            r#""Engineer":"Elvira Martuuk","EngineerID":300100,"Progress":"Barred""#,
            r#""Engineer":"Felicity Farseer","EngineerID":300160,"Progress":"Barred""#,
            r#""Engineer":"Felicity Farseer","EngineerID":999,"Progress":"Barred""#,
        ] {
            assert!(book.apply_event(&progress(body)).is_empty(), "{body}");
        }
        let e = book.status(300100).unwrap();
        assert_eq!((e.progress, e.rank), (EngineerStatus::Unlocked, Some(3)));
        assert!(book.status(300160).is_none());
        assert!(book.status(999).is_none());
        // También dentro de una lista de arranque.
        book.apply_event(&progress(
            r#""Engineers":[{"Engineer":"Elvira Martuuk","EngineerID":300100,"Progress":"Barred"},{"Engineer":"Elvira Martuuk","EngineerID":300160,"Progress":"Known"}]"#,
        ));
        assert!(book.status(300100).is_none());
        assert_eq!(book.status(300160).unwrap().progress, EngineerStatus::Known);
    }
}
