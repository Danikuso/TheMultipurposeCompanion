//! Avisos de misiones: caducidad, redirección, fallo y carga que falta.
//!
//! Fallida y redirigida salen del [`BookChange`] de `MissionBook::apply_event`;
//! `evaluate_alerts` solo trata las de tiempo y carga. Quien llama anota en
//! `already` las claves [`MissionAlert::key`] ya avisadas.
//!
//! Semántica: `ExpiringSoon` avisa una vez por caducidad (si la caducidad cambia,
//! vuelve a avisar). `CargoMissing` solo se avisa al estar el comandante en el sistema
//! (y estación, si ambas se conocen) del destino de una entrega, con menos carga a
//! bordo que lo restante (`total - entregado`); la clave incluye el déficit, así que
//! un déficit distinto vuelve a avisar.

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
    /// Distingue repeticiones: caducidad (segundos Unix) o déficit de carga; 0 en el resto.
    pub bucket: i64,
}

impl MissionAlert {
    /// Clave para el conjunto `already`.
    pub fn key(&self) -> (u64, u8, i64) {
        (self.mission_id, self.kind.code(), self.bucket)
    }
}

/// Dónde está el comandante ahora.
#[derive(Debug, Clone, Copy, Default)]
pub struct Here<'a> {
    pub system: Option<&'a str>,
    pub station: Option<&'a str>,
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
        bucket: 0,
    })
}

/// Avisos por tiempo (caduca en menos de 1 h) y carga (entrega pendiente sin cubrir
/// estando en el destino).
pub fn evaluate_alerts(
    book: &MissionBook,
    cargo: &[CargoItem],
    now: DateTime<Utc>,
    here: Here<'_>,
    already: &HashSet<(u64, u8, i64)>,
) -> Vec<MissionAlert> {
    let mut out = Vec::new();
    let same = |a: &str, b: &str| a.trim().eq_ignore_ascii_case(b.trim());
    for m in book.active() {
        if let Some(exp) = m.expiry
            && exp > now
            && exp - now < Duration::hours(1)
        {
            let a = MissionAlert {
                mission_id: m.id,
                kind: AlertKind::ExpiringSoon,
                at: now,
                bucket: exp.timestamp(),
            };
            if !already.contains(&a.key()) {
                out.push(a);
            }
        }
        if m.kind != MissionKind::Delivery {
            continue;
        }
        let (Some(total), Some(commodity)) = (m.progress.total, m.commodity.as_ref()) else {
            continue;
        };
        let at_dest = match (here.system, m.destination_system.as_deref()) {
            (Some(s), Some(d)) if same(s, d) => {
                match (here.station, m.destination_station.as_deref()) {
                    (Some(hs), Some(ds)) => same(hs, ds),
                    _ => true,
                }
            }
            _ => false,
        };
        if !at_dest {
            continue;
        }
        let pending = total.saturating_sub(m.progress.delivered);
        let aboard: u32 = cargo
            .iter()
            .filter(|c| c.mission_id == Some(m.id))
            .map(|c| c.count)
            .sum();
        let missing = pending.saturating_sub(aboard);
        if missing > 0 {
            let a = MissionAlert {
                mission_id: m.id,
                kind: AlertKind::CargoMissing {
                    commodity: commodity.clone(),
                    missing,
                },
                at: now,
                bucket: i64::from(missing),
            };
            if !already.contains(&a.key()) {
                out.push(a);
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
    use tmc_model::events::{CargoDepot, EventKind, MissionAccepted};

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
    fn eval(
        book: &MissionBook,
        cargo: &[CargoItem],
        here: Here<'_>,
        al: &HashSet<(u64, u8, i64)>,
    ) -> Vec<MissionAlert> {
        evaluate_alerts(book, cargo, now(), here, al)
    }
    fn lave() -> Here<'static> {
        Here {
            system: Some("Lave"),
            station: None,
        }
    }
    fn delivery(count: u32) -> MissionAccepted {
        let mut m = acc(9, "Mission_Delivery");
        m.count = Some(count);
        m.commodity = Some("$Gold_Name;".into());
        m.commodity_localised = Some("Gold".into());
        m.destination_system = Some("Lave".into());
        m
    }
    fn gold(n: u32) -> Vec<CargoItem> {
        vec![CargoItem {
            name: "gold".into(),
            count: n,
            mission_id: Some(9),
            ..Default::default()
        }]
    }
    fn deliver(book: &mut MissionBook, done: u32, total: u32) {
        book.apply_event(
            &EventKind::CargoDepot(CargoDepot {
                mission_id: 9,
                update_type: "Deliver".into(),
                items_delivered: done,
                total_items_to_deliver: total,
                ..Default::default()
            }),
            now(),
        );
    }

    #[test]
    fn expiring_under_one_hour_alerts_once_and_again_if_expiry_changes() {
        let mut m = acc(1, "Mission_Courier");
        m.expiry = Some(now() + Duration::minutes(59));
        let book = book_with(m);
        let mut already = HashSet::new();
        let a = eval(&book, &[], Here::default(), &already);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].kind, AlertKind::ExpiringSoon);
        already.insert(a[0].key());
        assert!(eval(&book, &[], Here::default(), &already).is_empty());
        let mut m = acc(1, "Mission_Courier");
        m.expiry = Some(now() + Duration::minutes(30));
        let book = book_with(m);
        assert_eq!(eval(&book, &[], Here::default(), &already).len(), 1);
    }

    #[test]
    fn expiry_edges_exact_hour_expired_and_none() {
        for exp in [
            Some(now() + Duration::minutes(60)),
            Some(now() - Duration::minutes(1)),
            None,
        ] {
            let mut m = acc(1, "Mission_Courier");
            m.expiry = exp;
            let book = book_with(m);
            assert!(eval(&book, &[], Here::default(), &HashSet::new()).is_empty());
        }
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
        assert!(eval(&book, &[], Here::default(), &HashSet::new()).is_empty());
    }

    #[test]
    fn missing_cargo_alert_only_at_destination_with_commodity() {
        let mut book = book_with(delivery(60));
        deliver(&mut book, 30, 60);
        let mut already = HashSet::new();
        // Lejos del destino: sin aviso.
        assert!(eval(&book, &gold(20), Here::default(), &already).is_empty());
        let a = eval(&book, &gold(20), lave(), &already);
        assert_eq!(
            a[0].kind,
            AlertKind::CargoMissing {
                commodity: "Gold".into(),
                missing: 10
            }
        );
        already.insert(a[0].key());
        assert!(eval(&book, &gold(20), lave(), &already).is_empty());
        // Otro déficit: vuelve a avisar.
        assert_eq!(eval(&book, &gold(10), lave(), &already).len(), 1);
    }

    #[test]
    fn enough_cargo_or_all_delivered_no_cargo_alert() {
        let mut book = book_with(delivery(60));
        deliver(&mut book, 30, 60);
        assert!(eval(&book, &gold(30), lave(), &HashSet::new()).is_empty());
        deliver(&mut book, 60, 60);
        assert!(eval(&book, &[], lave(), &HashSet::new()).is_empty());
        deliver(&mut book, 70, 60);
        assert!(eval(&book, &[], lave(), &HashSet::new()).is_empty());
    }
}
