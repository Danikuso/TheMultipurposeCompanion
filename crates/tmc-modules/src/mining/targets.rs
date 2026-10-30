//! Gestor de objetivos de yacimientos de minería, cálculo de ruta triangular
//! y venta óptima para múltiples minerales en bodega.

use crate::data::mining_hotspots;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tmc_model::net::{
    MiningLoopPlan, MiningMultiSellHit, MiningRingTarget, MiningSellStation, MiningTargetQuery,
    PadSize,
};

/// Estado del tema `mining.targets`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningTargetsTopic {
    pub status: String,
    pub commodity: String,
    pub query: MiningTargetQuery,
    pub targets: Vec<MiningRingTarget>,
}

/// Estado del tema `mining.loop`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningLoopTopic {
    pub status: String,
    pub plan: Option<MiningLoopPlan>,
}

/// Mineral en bodega del jugador con cantidad.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CargoMineralItem {
    pub name: String,
    pub count: u32,
}

/// Estado del tema `mining.cargo_sell`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningCargoSellTopic {
    pub status: String,
    pub commodities_in_cargo: Vec<CargoMineralItem>,
    pub best_multi_stations: Vec<MiningMultiSellHit>,
}

/// Lista canónica de minerales de minería comercializables.
pub const MINEABLE_COMMODITIES: &[&str] = &[
    "Platinum",
    "Painite",
    "Low Temperature Diamonds",
    "Bromellite",
    "Musgravite",
    "Monazite",
    "Alexandrite",
    "Void Opal",
    "Grandidierite",
    "Osmium",
    "Benitoite",
    "Serendibite",
    "Rhodplumsite",
    "Bauxite",
    "Cobalt",
    "Coltan",
    "Gallite",
    "Gold",
    "Indite",
    "Lepidolite",
    "Lithium",
    "Praseodymium",
    "Samarium",
    "Silver",
    "Tritium",
    "Uraninite",
];

/// Comprueba si una mercancía dada es un mineral o metal de minería.
pub fn is_mineable_commodity(name: &str) -> bool {
    let lower = name.trim().to_lowercase();
    MINEABLE_COMMODITIES
        .iter()
        .any(|m| m.to_lowercase() == lower)
}

/// Gestor de objetivos de minería, bucles y venta en bodega.
#[derive(Debug, Clone, Default)]
pub struct MiningTargetsManager {
    pub active_query: Option<MiningTargetQuery>,
    pub targets: Vec<MiningRingTarget>,
    pub selected_target: Option<MiningRingTarget>,
    pub loop_plan: Option<MiningLoopPlan>,
    pub cargo_multi_hits: Vec<MiningMultiSellHit>,
    pub cargo_minerals: Vec<CargoMineralItem>,
    pub targets_status: &'static str,
}

impl MiningTargetsManager {
    pub fn new() -> Self {
        Self {
            active_query: None,
            targets: Vec::new(),
            selected_target: None,
            loop_plan: None,
            cargo_multi_hits: Vec::new(),
            cargo_minerals: Vec::new(),
            targets_status: "idle",
        }
    }

    /// Realiza la búsqueda inicial e instantánea en el catálogo offline curado.
    pub fn search_offline(
        &mut self,
        query: MiningTargetQuery,
        reference_coords: Option<[f64; 3]>,
    ) -> bool {
        self.active_query = Some(query.clone());
        let cat = mining_hotspots();

        let commodity_lower = query.commodity.trim().to_lowercase();
        let ref_coords = query.reference_coords.or(reference_coords);

        let mut matches: Vec<MiningRingTarget> = cat
            .hotspots
            .iter()
            .filter(|h| {
                if !h.commodity.eq_ignore_ascii_case(&commodity_lower) {
                    return false;
                }
                if h.hotspot_count < query.min_hotspots {
                    return false;
                }
                if query.require_res_combo && h.res_intensity.is_none() {
                    return false;
                }
                true
            })
            .map(|h| {
                let dist = if let Some([x0, y0, z0]) = ref_coords {
                    let dx = h.coords[0] - x0;
                    let dy = h.coords[1] - y0;
                    let dz = h.coords[2] - z0;
                    (dx * dx + dy * dy + dz * dz).sqrt()
                } else {
                    0.0
                };

                MiningRingTarget {
                    system_name: h.system_name.clone(),
                    distance_ly: (dist * 10.0).round() / 10.0,
                    body_name: h.body_name.clone(),
                    ring_name: h.ring_name.clone(),
                    ring_type: h.ring_type.clone(),
                    hotspot_count: h.hotspot_count,
                    res_intensity: h.res_intensity.clone(),
                    res_distance_km: None,
                    arrival_distance_ls: None,
                    comment: h.comment.clone(),
                    other_hotspots: Vec::new(),
                    nearest_best_sell: None,
                }
            })
            .collect();

        matches.sort_by(|a, b| {
            a.distance_ly
                .partial_cmp(&b.distance_ly)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        self.targets = matches;
        self.targets_status = if self.targets.is_empty() {
            "empty"
        } else {
            "ok"
        };

        // Si hay resultados y no hay seleccionado, auto-selecciona el más cercano
        if !self.targets.is_empty() {
            self.selected_target = Some(self.targets[0].clone());
        } else {
            self.selected_target = None;
            self.loop_plan = None;
        }

        true
    }

    /// Fusiona los resultados remotos obtenidos de Spansh con el catálogo offline.
    pub fn merge_online_targets(&mut self, mut online_targets: Vec<MiningRingTarget>) -> bool {
        let cat = mining_hotspots();

        // Enriquecer cada resultado de Spansh con comentarios o RES del catálogo curado si coincide
        for target in &mut online_targets {
            if let Some(curated) = cat.hotspots.iter().find(|h| {
                h.system_name.eq_ignore_ascii_case(&target.system_name)
                    && h.ring_name.eq_ignore_ascii_case(&target.ring_name)
            }) {
                if target.comment.is_none() {
                    target.comment = curated.comment.clone();
                }
                if target.res_intensity.is_none() {
                    target.res_intensity = curated.res_intensity.clone();
                }
            }
        }

        online_targets.sort_by(|a, b| {
            a.distance_ly
                .partial_cmp(&b.distance_ly)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        self.targets = online_targets;
        self.targets_status = if self.targets.is_empty() {
            "empty"
        } else {
            "ok"
        };

        // Mantener selección o apuntar al primero
        if let Some(prev) = &self.selected_target {
            if let Some(matched) = self.targets.iter().find(|t| {
                t.system_name.eq_ignore_ascii_case(&prev.system_name)
                    && t.body_name.eq_ignore_ascii_case(&prev.body_name)
                    && t.ring_name.eq_ignore_ascii_case(&prev.ring_name)
            }) {
                self.selected_target = Some(matched.clone());
            } else if !self.targets.is_empty() {
                self.selected_target = Some(self.targets[0].clone());
            } else {
                self.selected_target = None;
                self.loop_plan = None;
            }
        } else if !self.targets.is_empty() {
            self.selected_target = Some(self.targets[0].clone());
        }

        true
    }

    /// Selecciona explícitamente un yacimiento por sistema, cuerpo y anillo.
    pub fn select_target(&mut self, system_name: &str, body_name: &str, ring_name: &str) -> bool {
        let found = self.targets.iter().find(|t| {
            t.system_name.eq_ignore_ascii_case(system_name)
                && t.body_name.eq_ignore_ascii_case(body_name)
                && t.ring_name.eq_ignore_ascii_case(ring_name)
        });

        if let Some(target) = found {
            self.selected_target = Some(target.clone());
            true
        } else {
            false
        }
    }

    /// Limpia la consulta activa y todos los resultados.
    pub fn clear(&mut self) {
        self.active_query = None;
        self.targets.clear();
        self.selected_target = None;
        self.loop_plan = None;
        self.targets_status = "idle";
    }

    /// Actualiza el plan de bucle triangular cuando se reciben estaciones compradoras.
    pub fn update_loop_with_sell_stations(
        &mut self,
        origin_system: &str,
        _origin_coords: Option<[f64; 3]>,
        sell_stations: &[MiningSellStation],
        cargo_qty: u32,
    ) -> bool {
        let Some(ring) = &self.selected_target else {
            self.loop_plan = None;
            return false;
        };

        if sell_stations.is_empty() {
            self.loop_plan = None;
            return false;
        }

        // Buscar la estación con el mejor balance (menor desvío y precio óptimo)
        let mut best_station = &sell_stations[0];
        let mut best_total_dist = f64::MAX;

        for st in sell_stations {
            let total = ring.distance_ly + st.distance_from_ring_ly;
            if total < best_total_dist {
                best_total_dist = total;
                best_station = st;
            }
        }

        let mut st_clone = best_station.clone();
        // Alerta de penalización de demanda: si cargo_qty > 25% de la demanda
        if st_clone.demand > 0 && cargo_qty > (st_clone.demand as f64 * 0.25) as u32 {
            st_clone.demand_tax_risk = true;
        }

        self.loop_plan = Some(MiningLoopPlan {
            origin_system: origin_system.to_string(),
            ring_target: ring.clone(),
            sell_station: st_clone,
            total_distance_ly: (best_total_dist * 10.0).round() / 10.0,
        });

        true
    }

    /// Actualiza la lista de minerales en bodega y las estaciones que compran el lote combinado.
    pub fn update_cargo_sell(
        &mut self,
        inventory: &BTreeMap<String, u32>,
        multi_hits: Vec<MiningMultiSellHit>,
    ) -> bool {
        let mut minerals = Vec::new();
        for (name, count) in inventory {
            if *count > 0 && is_mineable_commodity(name) {
                minerals.push(CargoMineralItem {
                    name: name.clone(),
                    count: *count,
                });
            }
        }

        self.cargo_minerals = minerals;
        self.cargo_multi_hits = multi_hits;
        true
    }

    /// Construye el tema `mining.targets`.
    pub fn build_targets_topic(&self) -> MiningTargetsTopic {
        MiningTargetsTopic {
            status: self.targets_status.to_string(),
            commodity: self
                .active_query
                .as_ref()
                .map(|q| q.commodity.clone())
                .unwrap_or_default(),
            query: self.active_query.clone().unwrap_or(MiningTargetQuery {
                reference_system: "".into(),
                reference_coords: None,
                commodity: "".into(),
                min_hotspots: 1,
                require_res_combo: false,
                populated_only: false,
                min_pad: PadSize::Small,
            }),
            targets: self.targets.clone(),
        }
    }

    /// Construye el tema `mining.loop`.
    pub fn build_loop_topic(&self) -> MiningLoopTopic {
        MiningLoopTopic {
            status: if self.loop_plan.is_some() {
                "ok".into()
            } else {
                "idle".into()
            },
            plan: self.loop_plan.clone(),
        }
    }

    /// Construye el tema `mining.cargo_sell`.
    pub fn build_cargo_sell_topic(&self) -> MiningCargoSellTopic {
        MiningCargoSellTopic {
            status: if !self.cargo_multi_hits.is_empty() {
                "ok".into()
            } else {
                "idle".into()
            },
            commodities_in_cargo: self.cargo_minerals.clone(),
            best_multi_stations: self.cargo_multi_hits.clone(),
        }
    }
}
