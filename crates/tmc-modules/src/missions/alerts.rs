//! Avisos de misiones: caducidad, redirección, fallo y carga que falta.
//!
//! Fallida y redirigida salen del [`BookChange`] de `MissionBook::apply_event`;
//! `evaluate_alerts` solo trata las de tiempo y carga. Quien llama anota en
//! `already` los pares `(misión, AlertKind::code())` ya avisados.

use super::book::{BookChange, MissionBook, MissionKind};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use std::collections::HashSet;
use tmc_model::events::CargoItem;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum AlertKind {
    ExpiringSoon,
    Redirected {
        system: String,
        station: Option<String>,
    },
    Failed,
    CargoMissing {
        commodity: String,
        missing: u32,
    },
}

impl AlertKind {
    /// Código estable para el conjunto `already`.
    pub fn code(&self) -> u8 {
        match self {
            AlertKind::ExpiringSoon => 0,
            AlertKind::Redirected { .. } => 1,
            AlertKind::Failed => 2,
            AlertKind::CargoMissing { .. } => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MissionAlert {
    pub mission_id: u64,
    pub kind: AlertKind,
    pub at: DateTime<Utc>,
}

/// Aviso derivado de un cambio del libro (fallida o redirigida).
pub fn alert_from_change(change: &BookChange, at: DateTime<Utc>) -> Option<MissionAlert> {
    let (mission_id, kind) = match change {
        BookChange::Failed(id) => (*id, AlertKind::Failed),
        BookChange::Redirected {
            id,
            system,
            station,
        } => (
            *id,
            AlertKind::Redirected {
                system: system.clone(),
                station: station.clone(),
            },
        ),
        _ => return None,
    };
    Some(MissionAlert {
        mission_id,
        kind,
        at,
    })
}

/// Avisos por tiempo (caduca en menos de 1 h) y carga (entrega pendiente sin cubrir).
pub fn evaluate_alerts(
    book: &MissionBook,
    cargo: &[CargoItem],
    now: DateTime<Utc>,
    already: &HashSet<(u64, u8)>,
) -> Vec<MissionAlert> {
    let mut out = Vec::new();
    for m in book.active() {
        if let Some(exp) = m.expiry
            && exp > now
            && exp - now < Duration::hours(1)
            && !already.contains(&(m.id, AlertKind::ExpiringSoon.code()))
        {
            out.push(MissionAlert {
                mission_id: m.id,
                kind: AlertKind::ExpiringSoon,
                at: now,
            });
        }
        if m.kind == MissionKind::Delivery
            && let Some(total) = m.progress.total
        {
            let pending = total.saturating_sub(m.progress.delivered);
            let mine: Vec<&CargoItem> = cargo
                .iter()
                .filter(|c| c.mission_id == Some(m.id))
                .collect();
            let aboard: u32 = mine.iter().map(|c| c.count).sum();
            let missing = pending.saturating_sub(aboard);
            if missing > 0 && !already.contains(&(m.id, 3)) {
                let commodity = mine.first().map(|c| c.name.clone()).unwrap_or_default();
                out.push(MissionAlert {
                    mission_id: m.id,
                    kind: AlertKind::CargoMissing { commodity, missing },
                    at: now,
                });
            }
        }
    }
    if !out.is_empty() {
        decide!("missions.alerts", { "count": out.len() });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tmc_model::events::{EventKind, MissionAccepted};

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 20, 12, 0, 0).unwrap()
    }
    fn book_with(m: MissionAccepted) -> MissionBook {
        let mut b = MissionBook::default();
        b.apply_event(&EventKind::MissionAccepted(Box::new(m)), now());
        b
    }
    fn acc(id: u64, name: &str) -> MissionAccepted {
        MissionAccepted {
            mission_id: id,
            name: name.into(),
            ..Default::default()
        }
    }

    #[test]
    fn expiring_under_one_hour_alerts_once() {
        let mut m = acc(1, "Mission_Courier");
        m.expiry = Some(now() + Duration::minutes(59));
        let book = book_with(m);
        let mut already = HashSet::new();
        let a = evaluate_alerts(&book, &[], now(), &already);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].kind, AlertKind::ExpiringSoon);
        already.insert((a[0].mission_id, a[0].kind.code()));
        assert!(evaluate_alerts(&book, &[], now(), &already).is_empty());
    }

    #[test]
    fn redirected_and_failed_alert_from_book_changes() {
        let r = BookChange::Redirected {
            id: 7,
            system: "Lave".into(),
            station: None,
        };
        let a = alert_from_change(&r, now()).unwrap();
        assert_eq!(
            a.kind,
            AlertKind::Redirected {
                system: "Lave".into(),
                station: None
            }
        );
        assert_eq!(
            alert_from_change(&BookChange::Failed(7), now())
                .unwrap()
                .kind,
            AlertKind::Failed
        );
        assert!(alert_from_change(&BookChange::Added(7), now()).is_none());
        let mut m = acc(7, "Mission_Courier");
        m.expiry = Some(now() + Duration::hours(5));
        let book = book_with(m);
        assert!(evaluate_alerts(&book, &[], now(), &HashSet::new()).is_empty());
    }

    #[test]
    fn missing_cargo_alert() {
        let mut m = acc(9, "Mission_Delivery");
        m.count = Some(60);
        let mut book = book_with(m);
        book.apply_event(
            &EventKind::CargoDepot(tmc_model::events::CargoDepot {
                mission_id: 9,
                items_delivered: 30,
                total_items_to_deliver: 60,
                ..Default::default()
            }),
            now(),
        );
        let cargo = [CargoItem {
            name: "gold".into(),
            count: 20,
            mission_id: Some(9),
            ..Default::default()
        }];
        let a = evaluate_alerts(&book, &cargo, now(), &HashSet::new());
        assert_eq!(
            a[0].kind,
            AlertKind::CargoMissing {
                commodity: "gold".into(),
                missing: 10
            }
        );
    }
}
