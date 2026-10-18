//! Constantes AX. Task 15 añade `AxTracker`.

/// Símbolos de `ShipTargeted.Ship` thargoides. NO VERIFICADO: viene de fuentes secundarias
/// (ver `docs/research/notes/spike_ax_f2.md` §4); los valores desconocidos se toleran.
pub const THARGOID_SHIPS: &[&str] = &[
    "scout", "scout_hq", "scout_q", "scout_c", "scout_nq", "cyclops", "basilisk", "medusa",
    "hydra", "scythe", "glaive", "orthrus",
];

/// `true` si el símbolo de nave (en minúsculas o no) es thargoide.
pub fn is_thargoid_ship(key: &str) -> bool {
    let k = key.trim().to_ascii_lowercase();
    THARGOID_SHIPS.contains(&k.as_str())
}

/// `true` si la facción es la thargoide (`$faction_Thargoid;`). NO VERIFICADO.
pub fn is_thargoid_faction(f: &str) -> bool {
    f.trim().eq_ignore_ascii_case("$faction_Thargoid;")
}
