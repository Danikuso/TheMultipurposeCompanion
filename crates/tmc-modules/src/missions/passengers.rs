//! Pasajeros a bordo y cabinas por clase.
//!
//! El diario no indica la clase de cada pasajero (spike): la ocupación por clase
//! queda en `None` y se muestra el agregado (`occupied_total` frente a `seats_total`).

use super::book::MissionBook;
use crate::data::{CabinClass, cabin_spec};
use serde::Serialize;
use std::collections::BTreeMap;
use tmc_model::events::{LoadoutModule, Passengers};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CabinOccupancy {
    /// `None` mientras el diario no permita conocer la clase de los pasajeros.
    pub occupied: Option<u32>,
    pub seats: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Cabins {
    pub by_class: BTreeMap<CabinClass, CabinOccupancy>,
    pub occupied_total: u32,
    pub seats_total: u32,
}

impl Cabins {
    /// Plazas por clase según el equipamiento; símbolos desconocidos se ignoran.
    pub fn from_loadout(modules: &[LoadoutModule]) -> Cabins {
        let mut c = Cabins::default();
        for m in modules {
            let Some(item) = m.item.as_deref() else {
                continue;
            };
            let Some(spec) = cabin_spec(&item.to_ascii_lowercase()) else {
                continue;
            };
            c.by_class
                .entry(spec.class)
                .or_insert(CabinOccupancy {
                    occupied: None,
                    seats: 0,
                })
                .seats += spec.seats;
            c.seats_total += spec.seats;
        }
        c
    }

    /// Total por manifiesto; por clase no hay dato, así que sigue en `None`.
    pub fn apply_manifest(&mut self, p: &Passengers) {
        self.occupied_total = p.manifest.iter().map(|e| e.count).sum();
        for o in self.by_class.values_mut() {
            o.occupied = None;
        }
        decide!("missions.cabins", {
            "occupied": self.occupied_total,
            "seats": self.seats_total,
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PassengerGroup {
    pub destination: String,
    pub count: u32,
    pub vip: bool,
    pub wanted: bool,
    pub mission_id: u64,
}

/// Agrupa el manifiesto por destino (el de la misión en el libro; vacío si se desconoce).
pub fn groups(book: &MissionBook, p: &Passengers) -> Vec<PassengerGroup> {
    let mut by_dest: BTreeMap<String, PassengerGroup> = BTreeMap::new();
    for e in &p.manifest {
        let destination = book
            .get(e.mission_id)
            .and_then(|m| m.destination_system.clone())
            .unwrap_or_default();
        let g = by_dest
            .entry(destination.clone())
            .or_insert_with(|| PassengerGroup {
                destination,
                count: 0,
                vip: false,
                wanted: false,
                mission_id: e.mission_id,
            });
        g.count += e.count;
        g.vip |= e.vip;
        g.wanted |= e.wanted;
        g.mission_id = g.mission_id.min(e.mission_id);
    }
    by_dest.into_values().collect()
}

pub fn wanted_aboard(p: &Passengers) -> bool {
    p.manifest.iter().any(|e| e.wanted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tmc_model::events::{EventKind, MissionAccepted, PassengerEntry};

    fn module(item: &str) -> LoadoutModule {
        LoadoutModule {
            item: Some(item.into()),
            ..Default::default()
        }
    }
    fn entry(id: u64, count: u32, wanted: bool) -> PassengerEntry {
        PassengerEntry {
            mission_id: id,
            count,
            wanted,
            ..Default::default()
        }
    }
    fn manifest(v: Vec<PassengerEntry>) -> Passengers {
        Passengers { manifest: v }
    }

    #[test]
    fn cabins_from_loadout() {
        let c = Cabins::from_loadout(&[
            module("Int_PassengerCabin_Size4_Class1"),
            module("int_passengercabin_size5_class4"),
        ]);
        assert_eq!(c.seats_total, 12);
        assert_eq!(c.by_class[&CabinClass::Luxury].seats, 4);
        assert_eq!(c.by_class[&CabinClass::Economy].seats, 8);
    }

    #[test]
    fn manifest_sets_occupancy() {
        let mut c = Cabins::from_loadout(&[module("int_passengercabin_size4_class1")]);
        c.apply_manifest(&manifest(vec![entry(1, 3, false), entry(2, 4, false)]));
        assert_eq!(c.occupied_total, 7);
        assert_eq!(c.by_class[&CabinClass::Economy].occupied, None);
    }

    #[test]
    fn wanted_passenger_flag() {
        assert!(!wanted_aboard(&manifest(vec![entry(1, 3, false)])));
        assert!(wanted_aboard(&manifest(vec![
            entry(1, 3, false),
            entry(2, 1, true)
        ])));
    }

    #[test]
    fn groups_by_destination() {
        let mut book = MissionBook::default();
        for (id, dest) in [(1, "Lave"), (2, "Lave"), (3, "Leesti")] {
            book.apply_event(
                &EventKind::MissionAccepted(Box::new(MissionAccepted {
                    mission_id: id,
                    name: "Mission_Passenger".into(),
                    destination_system: Some(dest.into()),
                    ..Default::default()
                })),
                Utc::now(),
            );
        }
        let g = groups(
            &book,
            &manifest(vec![
                entry(1, 3, false),
                entry(2, 2, true),
                entry(3, 4, false),
            ]),
        );
        assert_eq!(g.len(), 2);
        let lave = g.iter().find(|g| g.destination == "Lave").unwrap();
        assert_eq!(lave.count, 5);
        assert!(lave.wanted);
    }

    #[test]
    fn unknown_cabin_symbol_ignored() {
        let c = Cabins::from_loadout(&[module("int_passengercabin_size9_class9")]);
        assert_eq!(c.seats_total, 0);
        assert!(c.by_class.is_empty());
    }
}
