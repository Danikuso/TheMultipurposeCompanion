//! Turismo: ruta óptima entre paradas, créditos por año luz y veredicto.
//!
//! Búsqueda exacta hasta [`EXACT_MAX_STOPS`] paradas; por encima, vecino más cercano
//! más 2-opt con reinicios deterministas (LCG propio, sin dependencia `rand`).
//! Las escrituras a la tabla propia llevan etiqueta `*.write` y no ocurren en replay.

use serde::Serialize;
use serde_json::{Value, json};
use tmc_core::{Migration, StoreError, StoreTask};

pub const EXACT_MAX_STOPS: usize = 8;
/// Estimaciones sin verificar (se muestran como ≈).
pub const SECONDS_PER_JUMP: f64 = 45.0;
pub const SECONDS_PER_STOP: f64 = 240.0;
pub const ESTIMATES_VERIFIED: bool = false;

pub const TAG_STATS_WRITE: &str = "missions.stats.write";
pub const TAG_STATS_COMPLETE_WRITE: &str = "missions.stats_complete.write";
pub const TAG_PASSENGER_STATS: &str = "missions.passenger_stats";
pub const STATUS_STOPS_UNAVAILABLE: &str = "stops_unavailable";

pub const PASSENGER_STATS_MIGRATION: Migration = Migration {
    id: "missions-001",
    sql: "CREATE TABLE missions_passenger_stats (mission_id INTEGER PRIMARY KEY, cmdr TEXT NOT NULL, accepted_at TEXT NOT NULL, reward INTEGER NOT NULL, route_ly REAL NOT NULL, completed_at TEXT);",
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Profitable,
    NotWorth,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TourismPlan {
    pub mission_id: u64,
    pub order: Vec<String>,
    pub board_order_ly: f64,
    pub optimal_ly: f64,
    pub jumps: Option<u32>,
    /// `None` si la ruta mide 0 al (veredicto rentable: sin coste de viaje).
    pub cr_per_ly: Option<f64>,
    /// Estimación (≈) con constantes sin verificar.
    pub cr_per_hour: Option<f64>,
    pub estimates_verified: bool,
    pub reference_cr_ly: f64,
    pub reference_is_own: bool,
    pub verdict: Verdict,
    pub can_abandon_now: bool,
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// Longitud del recorrido `start → stops (en el orden dado) → return_to`.
pub fn path_length(
    start: [f64; 3],
    stops_in_order: &[[f64; 3]],
    return_to: Option<[f64; 3]>,
) -> f64 {
    let mut cur = start;
    let mut total = 0.0;
    for s in stops_in_order {
        total += dist(cur, *s);
        cur = *s;
    }
    if let Some(r) = return_to {
        total += dist(cur, r);
    }
    total
}

fn len_of_perm(start: [f64; 3], stops: &[[f64; 3]], perm: &[usize], ret: Option<[f64; 3]>) -> f64 {
    let mut cur = start;
    let mut total = 0.0;
    for &i in perm {
        total += dist(cur, stops[i]);
        cur = stops[i];
    }
    if let Some(r) = ret {
        total += dist(cur, r);
    }
    total
}

/// Orden óptimo exacto por permutaciones (úsese con pocas paradas).
pub fn exact_order(
    start: [f64; 3],
    stops: &[[f64; 3]],
    return_to: Option<[f64; 3]>,
) -> (Vec<usize>, f64) {
    let n = stops.len();
    let mut perm: Vec<usize> = (0..n).collect();
    let mut best = perm.clone();
    let mut best_len = len_of_perm(start, stops, &perm, return_to);
    // Heap iterativo; solo mejora estricta.
    let mut c = vec![0usize; n];
    let mut i = 0;
    while i < n {
        if c[i] < i {
            if i % 2 == 0 {
                perm.swap(0, i);
            } else {
                perm.swap(c[i], i);
            }
            let l = len_of_perm(start, stops, &perm, return_to);
            if l < best_len - 1e-12 {
                best_len = l;
                best = perm.clone();
            }
            c[i] += 1;
            i = 0;
        } else {
            c[i] = 0;
            i += 1;
        }
    }
    (best, best_len)
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

fn two_opt(start: [f64; 3], stops: &[[f64; 3]], ret: Option<[f64; 3]>, perm: &mut [usize]) -> f64 {
    let n = perm.len();
    let mut best = len_of_perm(start, stops, perm, ret);
    let mut improved = true;
    while improved {
        improved = false;
        for i in 0..n {
            for j in i + 1..n {
                perm[i..=j].reverse();
                let l = len_of_perm(start, stops, perm, ret);
                if l < best - 1e-12 {
                    best = l;
                    improved = true;
                } else {
                    perm[i..=j].reverse();
                }
            }
        }
    }
    best
}

/// Vecino más cercano + 2-opt, con reinicios aleatorios deterministas (semilla fija).
pub fn heuristic_order(
    start: [f64; 3],
    stops: &[[f64; 3]],
    return_to: Option<[f64; 3]>,
) -> (Vec<usize>, f64) {
    let n = stops.len();
    let mut left: Vec<usize> = (0..n).collect();
    let mut perm = Vec::with_capacity(n);
    let mut cur = start;
    while !left.is_empty() {
        let mut k = 0;
        for (pos, &idx) in left.iter().enumerate() {
            if dist(cur, stops[idx]) < dist(cur, stops[left[k]]) {
                k = pos;
            }
        }
        let idx = left.remove(k);
        cur = stops[idx];
        perm.push(idx);
    }
    let mut best_len = two_opt(start, stops, return_to, &mut perm);
    let mut best = perm;
    let mut rng = Lcg(0x5EED);
    for _ in 0..8 {
        let mut p: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            p.swap(i, (rng.next() as usize) % (i + 1));
        }
        let l = two_opt(start, stops, return_to, &mut p);
        if l < best_len - 1e-12 {
            best_len = l;
            best = p;
        }
    }
    (best, best_len)
}

/// Exacto hasta [`EXACT_MAX_STOPS`]; heurístico por encima.
fn optimal_order(
    start: [f64; 3],
    stops: &[[f64; 3]],
    return_to: Option<[f64; 3]>,
) -> (Vec<usize>, f64) {
    if stops.len() <= EXACT_MAX_STOPS {
        exact_order(start, stops, return_to)
    } else {
        heuristic_order(start, stops, return_to)
    }
}

/// Media propia: Σ recompensa / Σ al de las completadas; `None` sin datos.
pub fn own_reference(rows: &[(u64, f64)]) -> Option<f64> {
    let ly: f64 = rows.iter().map(|r| r.1).sum();
    if rows.is_empty() || ly <= 0.0 {
        return None;
    }
    let reward: f64 = rows.iter().map(|r| r.0 as f64).sum();
    Some(reward / ly)
}

/// Entradas de [`plan`].
pub struct PlanInput<'a> {
    pub mission_id: u64,
    pub reward: u64,
    pub start: [f64; 3],
    /// Paradas en el orden del tablón: nombre y coordenadas.
    pub stops: &'a [(String, [f64; 3])],
    pub return_to: Option<[f64; 3]>,
    pub jump_range: Option<f64>,
    pub own_reference: Option<f64>,
    pub setting_reference: f64,
    pub can_abandon_now: bool,
}

fn estimate_jumps(start: [f64; 3], pts: &[[f64; 3]], ret: Option<[f64; 3]>, range: f64) -> u32 {
    let mut cur = start;
    let mut total = 0u32;
    let mut legs: Vec<[f64; 3]> = pts.to_vec();
    legs.extend(ret);
    for p in legs {
        let d = dist(cur, p);
        if d > 0.0 {
            total = total.saturating_add((d / range).ceil().max(1.0) as u32);
        }
        cur = p;
    }
    total
}

/// Calcula el plan; `Err("stops_unavailable")` si no hay paradas.
pub fn plan(input: &PlanInput) -> Result<TourismPlan, &'static str> {
    let finite = |p: &[f64; 3]| p.iter().all(|v| v.is_finite());
    if input.stops.is_empty()
        || !finite(&input.start)
        || input.stops.iter().any(|s| !finite(&s.1))
        || input.return_to.is_some_and(|r| !finite(&r))
    {
        return Err(STATUS_STOPS_UNAVAILABLE);
    }
    let coords: Vec<[f64; 3]> = input.stops.iter().map(|s| s.1).collect();
    let board_ly = path_length(input.start, &coords, input.return_to);
    let (order, optimal_ly) = optimal_order(input.start, &coords, input.return_to);
    let heuristic = coords.len() > EXACT_MAX_STOPS;
    let ordered: Vec<[f64; 3]> = order.iter().map(|&i| coords[i]).collect();
    let jumps = input
        .jump_range
        .filter(|r| r.is_finite() && *r > 0.0)
        .map(|r| estimate_jumps(input.start, &ordered, input.return_to, r));
    let cr_per_ly = (optimal_ly > 0.0).then(|| input.reward as f64 / optimal_ly);
    let cr_per_hour = jumps.map(|j| {
        let secs = j as f64 * SECONDS_PER_JUMP + coords.len() as f64 * SECONDS_PER_STOP;
        input.reward as f64 / (secs / 3600.0)
    });
    let valid = |r: f64| r.is_finite() && r > 0.0;
    let (reference, own) = match input.own_reference.filter(|r| valid(*r)) {
        Some(r) => (r, true),
        None if valid(input.setting_reference) => (input.setting_reference, false),
        None => (100_000.0, false),
    };
    // Ruta de 0 al con recompensa: no hay coste de viaje, se considera rentable.
    let verdict = match cr_per_ly {
        None if input.reward > 0 => Verdict::Profitable,
        Some(c) if c >= reference => Verdict::Profitable,
        _ => Verdict::NotWorth,
    };
    decide!("missions.tourism", {
        "stops": coords.len(),
        "heuristic": heuristic,
        "profitable": verdict == Verdict::Profitable,
    });
    Ok(TourismPlan {
        mission_id: input.mission_id,
        order: order.iter().map(|&i| input.stops[i].0.clone()).collect(),
        board_order_ly: board_ly,
        optimal_ly,
        jumps,
        cr_per_ly,
        cr_per_hour,
        estimates_verified: ESTIMATES_VERIFIED,
        reference_cr_ly: reference,
        reference_is_own: own,
        verdict,
        can_abandon_now: input.can_abandon_now,
    })
}

fn bad(e: impl std::fmt::Display) -> StoreError {
    StoreError::Unsupported(e.to_string())
}

/// Escritura (`missions.stats.write`) al aceptar; actualiza la ruta si se recalcula.
pub fn stats_write_task(
    cmdr: String,
    mission_id: u64,
    accepted_at: String,
    reward: u64,
    route_ly: f64,
) -> StoreTask {
    Box::new(move |store| {
        let id = i64::try_from(mission_id).map_err(bad)?;
        let r = i64::try_from(reward).map_err(bad)?;
        store.exec_json(
            "INSERT INTO missions_passenger_stats (mission_id, cmdr, accepted_at, reward, route_ly) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(mission_id) DO UPDATE SET route_ly = excluded.route_ly",
            &[
                json!(id),
                json!(cmdr),
                json!(accepted_at),
                json!(r),
                json!(route_ly),
            ],
        )?;
        Ok(json!({"mission_id": mission_id}))
    })
}

/// Escritura (`missions.stats_complete.write`) al completar la misión.
pub fn stats_complete_task(cmdr: String, mission_id: u64, at: String) -> StoreTask {
    Box::new(move |store| {
        let id = i64::try_from(mission_id).map_err(bad)?;
        store.exec_json(
            "UPDATE missions_passenger_stats SET completed_at = ?1 WHERE mission_id = ?2 AND cmdr = ?3",
            &[json!(at), json!(id), json!(cmdr)],
        )?;
        Ok(json!({"mission_id": mission_id}))
    })
}

/// Lectura (`missions.passenger_stats`) de las completadas del comandante.
pub fn stats_read_task(cmdr: String) -> StoreTask {
    Box::new(move |store| {
        let rows = store.query_json(
            "SELECT reward, route_ly FROM missions_passenger_stats \
             WHERE cmdr = ?1 AND completed_at IS NOT NULL AND route_ly > 0",
            &[json!(cmdr)],
        )?;
        Ok(json!({"cmdr": cmdr, "rows": rows}))
    })
}

/// Interpreta el resultado de [`stats_read_task`].
pub fn rows_from_result(result: &Value) -> Vec<(u64, f64)> {
    result["rows"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| Some((r[0].as_u64()?, r[1].as_f64()?)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmc_core::Store;

    fn lcg_points(seed: u64, n: usize) -> Vec<[f64; 3]> {
        let mut s = seed;
        let mut next = || {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((s >> 33) % 2000) as f64 / 10.0
        };
        (0..n).map(|_| [next(), next(), next()]).collect()
    }

    const O: [f64; 3] = [0.0; 3];

    #[test]
    fn optimal_is_le_board_order() {
        let stops = [[10.0, 0.0, 0.0], [-10.0, 0.0, 0.0], [11.0, 0.0, 0.0]];
        assert_eq!(path_length(O, &stops, None), 51.0);
        let (order, len) = optimal_order(O, &stops, None);
        assert_eq!(order, vec![1, 0, 2]);
        assert!((len - 31.0).abs() < 1e-9);
    }

    #[test]
    fn return_trip_included_when_required() {
        let (_, len) = optimal_order(O, &[[10.0, 0.0, 0.0]], Some(O));
        assert!((len - 20.0).abs() < 1e-9);
    }

    #[test]
    fn exact_and_heuristic_agree_on_small_case() {
        let pts = lcg_points(7, 6);
        let (_, e) = exact_order(O, &pts, None);
        let (_, h) = heuristic_order(O, &pts, None);
        assert!(h >= e - 1e-9);
        assert!(e <= path_length(O, &pts, None) + 1e-9);
    }

    #[test]
    fn nine_stops_use_heuristic_and_return_valid_permutation() {
        let pts = lcg_points(7, 9);
        let (order, len) = optimal_order(O, &pts, Some(O));
        let mut sorted = order.clone();
        sorted.sort();
        assert_eq!(sorted, (0..9).collect::<Vec<_>>());
        let ordered: Vec<_> = order.iter().map(|&i| pts[i]).collect();
        assert!((len - path_length(O, &ordered, Some(O))).abs() < 1e-9);
        assert!(len <= path_length(O, &pts, Some(O)) + 1e-9);
    }

    fn input(
        stops: &[(String, [f64; 3])],
        reward: u64,
        own: Option<f64>,
        setting: f64,
    ) -> PlanInput<'_> {
        PlanInput {
            mission_id: 1,
            reward,
            start: O,
            stops,
            return_to: None,
            jump_range: Some(10.0),
            own_reference: own,
            setting_reference: setting,
            can_abandon_now: true,
        }
    }

    #[test]
    fn cr_per_ly_and_verdict_vs_reference() {
        let stops = vec![("A".to_string(), [32.0, 0.0, 0.0])];
        let p = plan(&input(&stops, 3_200_000, None, 120_000.0)).unwrap();
        assert!((p.cr_per_ly.unwrap() - 100_000.0).abs() < 1e-6);
        assert_eq!(p.verdict, Verdict::NotWorth);
        assert!(!p.reference_is_own);
        assert_eq!(p.jumps, Some(4));
        assert!(p.cr_per_hour.is_some() && !p.estimates_verified);
        let p = plan(&input(&stops, 3_200_000, Some(80_000.0), 120_000.0)).unwrap();
        assert_eq!(p.verdict, Verdict::Profitable);
        assert!(p.reference_is_own);
    }

    #[test]
    fn own_reference_is_ratio_of_sums() {
        let r = own_reference(&[(1_000_000, 10.0), (500_000, 10.0)]).unwrap();
        assert!((r - 75_000.0).abs() < 1e-9);
        assert_eq!(own_reference(&[]), None);
    }

    #[test]
    fn no_stops_gives_unavailable() {
        assert_eq!(
            plan(&input(&[], 1, None, 1.0)).unwrap_err(),
            "stops_unavailable"
        );
    }

    #[test]
    fn stats_tasks_roundtrip() {
        let store = Store::open_in_memory().unwrap();
        store
            .apply_migrations(&[PASSENGER_STATS_MIGRATION])
            .unwrap();
        stats_write_task("C".into(), 5, "t".into(), 1_000_000, 10.0)(&store).unwrap();
        stats_write_task("C".into(), 6, "t".into(), 500_000, 10.0)(&store).unwrap();
        let none = rows_from_result(&stats_read_task("C".into())(&store).unwrap());
        assert!(none.is_empty());
        stats_complete_task("C".into(), 5, "t2".into())(&store).unwrap();
        stats_complete_task("C".into(), 6, "t2".into())(&store).unwrap();
        let rows = rows_from_result(&stats_read_task("C".into())(&store).unwrap());
        assert_eq!(own_reference(&rows), Some(75_000.0));
    }

    fn stop(x: f64) -> (String, [f64; 3]) {
        ("S".to_string(), [x, 0.0, 0.0])
    }

    #[test]
    fn non_finite_coordinates_are_unavailable() {
        for bad in [f64::NAN, f64::INFINITY] {
            let s = vec![stop(bad)];
            assert!(plan(&input(&s, 1, None, 1.0)).is_err());
        }
        let s = vec![stop(5.0)];
        let mut i = input(&s, 1, None, 1.0);
        i.start = [f64::NAN, 0.0, 0.0];
        assert!(plan(&i).is_err());
        let mut i = input(&s, 1, None, 1.0);
        i.return_to = Some([0.0, f64::INFINITY, 0.0]);
        assert!(plan(&i).is_err());
    }

    #[test]
    fn zero_distance_is_profitable_without_cr_per_ly() {
        let s = vec![stop(0.0), stop(0.0)];
        let p = plan(&input(&s, 500_000, None, 100_000.0)).unwrap();
        assert_eq!(p.cr_per_ly, None);
        assert_eq!(p.verdict, Verdict::Profitable);
        assert_eq!(p.jumps, Some(0));
        let p = plan(&input(&s, 0, None, 100_000.0)).unwrap();
        assert_eq!(p.verdict, Verdict::NotWorth);
    }

    #[test]
    fn invalid_jump_range_gives_no_jumps_or_hourly() {
        let s = vec![stop(10.0)];
        for r in [None, Some(0.0), Some(f64::NAN), Some(f64::INFINITY)] {
            let mut i = input(&s, 1_000, None, 1.0);
            i.jump_range = r;
            let p = plan(&i).unwrap();
            assert!(p.jumps.is_none() && p.cr_per_hour.is_none(), "{r:?}");
        }
    }

    #[test]
    fn invalid_references_fall_back() {
        let s = vec![stop(10.0)];
        let p = plan(&input(&s, 1_000_000, Some(f64::NAN), 50_000.0)).unwrap();
        assert!(!p.reference_is_own && p.reference_cr_ly == 50_000.0);
        let p = plan(&input(&s, 1_000_000, Some(0.0), f64::NAN)).unwrap();
        assert_eq!(p.reference_cr_ly, 100_000.0);
    }

    #[test]
    fn plan_includes_return_leg() {
        let s = vec![stop(10.0)];
        let mut i = input(&s, 2_000_000, None, 1.0);
        i.return_to = Some(O);
        let p = plan(&i).unwrap();
        assert!((p.optimal_ly - 20.0).abs() < 1e-9);
        assert_eq!(p.jumps, Some(2));
    }

    #[test]
    fn heuristic_matches_exact_on_several_instances() {
        for seed in 1..=6u64 {
            for n in [4usize, 6, 7] {
                let pts = lcg_points(seed, n);
                for ret in [None, Some(O)] {
                    let (_, e) = exact_order(O, &pts, ret);
                    let (ho, h) = heuristic_order(O, &pts, ret);
                    assert!(h >= e - 1e-9, "seed {seed} n {n}");
                    assert!(h <= e * 1.15 + 1e-9, "seed {seed} n {n}: {h} vs {e}");
                    assert_eq!(ho.len(), n);
                }
            }
        }
    }

    #[test]
    fn stats_write_is_idempotent_and_read_excludes_zero_route() {
        let store = Store::open_in_memory().unwrap();
        store
            .apply_migrations(&[PASSENGER_STATS_MIGRATION])
            .unwrap();
        for ly in [10.0, 12.0] {
            stats_write_task("C".into(), 5, "t".into(), 1_000_000, ly)(&store).unwrap();
        }
        stats_write_task("C".into(), 6, "t".into(), 900_000, 0.0)(&store).unwrap();
        let n = store
            .query_json("SELECT route_ly FROM missions_passenger_stats", &[])
            .unwrap();
        assert_eq!(n.len(), 2);
        stats_complete_task("C".into(), 5, "t2".into())(&store).unwrap();
        stats_complete_task("C".into(), 6, "t2".into())(&store).unwrap();
        let rows = rows_from_result(&stats_read_task("C".into())(&store).unwrap());
        assert_eq!(rows, vec![(1_000_000, 12.0)]);
    }

    #[test]
    fn reward_above_i64_max_is_an_error() {
        let store = Store::open_in_memory().unwrap();
        store
            .apply_migrations(&[PASSENGER_STATS_MIGRATION])
            .unwrap();
        assert!(stats_write_task("C".into(), 1, "t".into(), u64::MAX, 1.0)(&store).is_err());
        assert!(stats_write_task("C".into(), u64::MAX, "t".into(), 1, 1.0)(&store).is_err());
    }
}
