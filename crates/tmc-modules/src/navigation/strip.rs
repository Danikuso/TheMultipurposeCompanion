//! Tira de ruta: tipos de las fichas y construcción de la lista de sistemas.

use crate::data::StarSpecial;
use crate::navigation::route::{ActivePlan, RouteSource, pick_source};
use chrono::{DateTime, Utc};
use serde::Serialize;
use tmc_model::net::{NetSource, PlannedHop, RemoteSystem};
use tmc_model::state::GameState;

/// Sistemas que se muestran además del actual: siguiente + 3 posteriores.
pub const STRIP_AHEAD: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RemoteStatus {
    Pending,
    Ok,
    NoData,
    Disabled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RemoteState {
    pub status: RemoteStatus,
    pub source: Option<NetSource>,
    pub fetched_at: Option<DateTime<Utc>>,
    pub data: Option<RemoteSystem>,
}

impl RemoteState {
    pub fn pending() -> Self {
        Self::bare(RemoteStatus::Pending)
    }

    pub fn disabled() -> Self {
        Self::bare(RemoteStatus::Disabled)
    }

    fn bare(status: RemoteStatus) -> Self {
        Self {
            status,
            source: None,
            fetched_at: None,
            data: None,
        }
    }
}

/// Política local del sistema actual, tal como la informan `FSDJump`/`Location`.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct LocalPolitics {
    pub allegiance: Option<String>,
    pub government: Option<String>,
    pub economy: Option<String>,
    pub second_economy: Option<String>,
    pub security: Option<String>,
    pub population: Option<u64>,
    pub controlling_faction: Option<String>,
    pub faction_state: Option<String>,
    pub powers: Vec<String>,
    pub powerplay_state: Option<String>,
}

impl LocalPolitics {
    pub fn from_location(l: &tmc_model::events::LocationInfo) -> Option<Self> {
        let p = Self {
            allegiance: l.system_allegiance.clone().filter(|s| !s.is_empty()),
            government: l.system_government_localised.clone(),
            economy: l.system_economy_localised.clone(),
            second_economy: l.system_second_economy_localised.clone(),
            security: l.system_security_localised.clone(),
            population: l.population,
            controlling_faction: l.system_faction.as_ref().and_then(|f| f.name.clone()),
            faction_state: l
                .system_faction
                .as_ref()
                .and_then(|f| f.faction_state.clone()),
            powers: l.powers.clone(),
            powerplay_state: l.powerplay_state.clone(),
        };
        (p != Self::default()).then_some(p)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SystemCard {
    pub name: String,
    pub id64: Option<u64>,
    pub jump_ly: Option<f64>,
    pub star_class: Option<String>,
    pub scoopable: Option<bool>,
    pub special: Option<StarSpecial>,
    pub fuel_after_pct: Option<f64>,
    pub reachable: Option<bool>,
    pub visits: Option<u32>,
    pub remote: RemoteState,
    pub local_politics: Option<LocalPolitics>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RouteTopic {
    pub source: RouteSource,
    pub current: SystemCard,
    pub next: Option<SystemCard>,
    pub following: Vec<SystemCard>,
    pub remaining_jumps: Option<u32>,
    pub total_ly: Option<f64>,
    pub off_route: bool,
    pub copy_target: Option<String>,
    pub copy_seq: u64,
}

/// Sistema de la tira antes de enriquecerlo con datos locales y remotos.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub name: String,
    pub id64: Option<u64>,
    pub pos: Option<[f64; 3]>,
    pub star_class: Option<String>,
    /// Distancia desde el sistema anterior de la tira.
    pub leg_ly: Option<f64>,
    /// Saltos que agrupa el tramo (los tramos de neutrones pueden agrupar varios).
    pub jumps: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Strip {
    pub source: RouteSource,
    pub current: Entry,
    /// Solo los siguientes visibles (`STRIP_AHEAD`); los totales cubren toda la ruta.
    pub upcoming: Vec<Entry>,
    pub off_route: bool,
    pub remaining_jumps: Option<u32>,
    pub total_ly: Option<f64>,
}

impl Strip {
    /// Sistema actual y los `STRIP_AHEAD` siguientes, en orden.
    pub fn visible(&self) -> impl Iterator<Item = &Entry> {
        std::iter::once(&self.current).chain(self.upcoming.iter().take(STRIP_AHEAD))
    }
}

pub fn system_key(id64: u64) -> String {
    format!("system:{id64}")
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn plan_entry(h: &PlannedHop) -> Entry {
    Entry {
        name: h.name.clone(),
        id64: h.id64,
        pos: h.star_pos,
        star_class: h.star_class.clone(),
        leg_ly: Some(h.distance_ly).filter(|d| d.is_finite()),
        jumps: h.jumps,
    }
}

pub fn build_strip(plan: Option<&ActivePlan>, state: &GameState) -> Strip {
    let loc = &state.location;
    let mut current = Entry {
        name: loc.system.clone(),
        id64: (loc.system_address != 0).then_some(loc.system_address),
        pos: (!loc.system.is_empty()).then_some(loc.star_pos),
        star_class: None,
        leg_ly: None,
        jumps: 0,
    };
    let source = pick_source(plan, &state.route);
    match (source, plan) {
        (RouteSource::Plan, Some(p)) => {
            if let Some(h) = p.hops.get(p.current_index) {
                if current.name.is_empty() {
                    current.name = h.name.clone();
                    current.id64 = h.id64;
                    current.pos = h.star_pos;
                }
                if h.name.eq_ignore_ascii_case(&current.name) {
                    current.star_class = h.star_class.clone();
                }
            }
            let all = p.upcoming(usize::MAX);
            let upcoming: Vec<Entry> = all.iter().take(STRIP_AHEAD).map(plan_entry).collect();
            Strip {
                source,
                current,
                remaining_jumps: Some(all.iter().fold(0u32, |a, h| a.saturating_add(h.jumps))),
                total_ly: Some(p.remaining_ly()),
                off_route: p.off_route,
                upcoming,
            }
        }
        (RouteSource::NavRoute, _) => {
            let hops = &state.route.hops;
            let idx = hops.iter().position(|h| h.star_system == current.name);
            if let Some(i) = idx {
                current.star_class = hops[i].star_class.clone();
            }
            let mut prev = current.pos;
            let mut upcoming = Vec::new();
            let (mut count, mut total, mut any_leg) = (0usize, 0.0, false);
            for h in &hops[idx.map_or(0, |i| i + 1)..] {
                let pos = Some(h.star_pos);
                let leg_ly = prev.zip(pos).map(|(a, b)| distance(a, b));
                prev = pos;
                count += 1;
                if let Some(d) = leg_ly {
                    total += d;
                    any_leg = true;
                }
                if upcoming.len() < STRIP_AHEAD {
                    upcoming.push(Entry {
                        name: h.star_system.clone(),
                        id64: (h.system_address != 0).then_some(h.system_address),
                        pos,
                        star_class: h.star_class.clone(),
                        leg_ly,
                        jumps: 1,
                    });
                }
            }
            Strip {
                source,
                current,
                remaining_jumps: u32::try_from(count).ok(),
                total_ly: any_leg.then_some(total),
                off_route: false,
                upcoming,
            }
        }
        _ => Strip {
            source: RouteSource::Current,
            current,
            upcoming: Vec::new(),
            off_route: false,
            remaining_jumps: None,
            total_ly: None,
        },
    }
}
