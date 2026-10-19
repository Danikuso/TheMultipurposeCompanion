//! Filtro por coordenadas y distancia.

use crate::coords::{CoordIndex, dist};
use crate::decode::DropReason;
use std::collections::BTreeMap;
use tmc_model::market::MarketSnapshot;

#[derive(Debug, Clone, PartialEq)]
pub struct EddnFilter {
    pub enabled: bool,
    pub center: Option<[f64; 3]>,
    pub radius_ly: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EddnStats {
    pub received: u64,
    pub accepted: u64,
    pub dropped: BTreeMap<DropReason, u64>,
}

/// Rellena `star_pos` y descarta lo desconocido (`UnknownCoords`) o lejano (`Far`).
pub fn accept(
    snap: &mut MarketSnapshot,
    idx: &CoordIndex,
    f: &EddnFilter,
) -> Result<(), DropReason> {
    let pos = idx
        .get(&snap.system_name.to_lowercase())
        .ok_or(DropReason::UnknownCoords)?;
    snap.star_pos = Some(pos);
    if f.enabled {
        // Radio NaN o negativo: se rechaza todo (falla cerrado).
        if f.radius_ly.is_nan() || f.radius_ly < 0.0 {
            return Err(DropReason::Far);
        }
        if f.center.is_some_and(|c| dist(pos, c) > f.radius_ly) {
            return Err(DropReason::Far);
        }
    }
    Ok(())
}
