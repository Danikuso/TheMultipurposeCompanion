//! Modelos y eventos para Powerplay 2.0 (Ascendancy).

use serde::{Deserialize, Serialize};

/// Umbrales de méritos para los primeros rangos de Powerplay 2.0.
/// A partir de rangos superiores, la fórmula escala linealmente con un paso definido.
pub fn merits_threshold_for_rank(rank: u32) -> u32 {
    match rank {
        0 | 1 => 0,
        2 => 500,
        3 => 2_000,
        4 => 5_000,
        5 => 15_000,
        6 => 30_000,
        7 => 50_000,
        8 => 75_000,
        9 => 105_000,
        10 => 140_000,
        // Progresión continua para rangos altos (100+)
        r => 140_000 + (r - 10) * 40_000,
    }
}

/// Evento del diario: `Powerplay` con el estado de afiliación, rango y méritos acumulados.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Powerplay {
    pub power: String,
    pub rank: u32,
    pub merits: u32,
    pub votes: Option<u32>,
    pub time_pledged: Option<u64>,
}

impl Powerplay {
    /// Semanas completas de lealtad consecutiva con la potencia jurada.
    #[must_use]
    pub fn weeks_pledged(&self) -> u32 {
        self.time_pledged
            .map(|sec| (sec / 604_800) as u32)
            .unwrap_or(0)
    }

    /// Méritos requeridos para alcanzar el rango objetivo siguiente (`rank + 1`).
    #[must_use]
    pub fn next_rank_target_merits(&self) -> u32 {
        merits_threshold_for_rank(self.rank.saturating_add(1))
    }

    /// Méritos base del rango actual.
    #[must_use]
    pub fn current_rank_base_merits(&self) -> u32 {
        merits_threshold_for_rank(self.rank)
    }

    /// Méritos que aún faltan para subir al siguiente rango.
    #[must_use]
    pub fn merits_needed_for_next_rank(&self) -> u32 {
        let target = self.next_rank_target_merits();
        target.saturating_sub(self.merits)
    }

    /// Ratio de progreso dentro del rango actual hacia el siguiente (0.0 a 1.0).
    #[must_use]
    pub fn progress_to_next_rank(&self) -> f64 {
        let base = self.current_rank_base_merits();
        let target = self.next_rank_target_merits();
        if target <= base {
            return 1.0;
        }
        let current_in_tier = self.merits.saturating_sub(base);
        let tier_span = target - base;
        (current_in_tier as f64 / tier_span as f64).clamp(0.0, 1.0)
    }

    /// Alias para `progress_to_next_rank`.
    #[must_use]
    pub fn progress_ratio(&self) -> f64 {
        self.progress_to_next_rank()
    }

    /// Porcentaje de progreso hacia el siguiente rango de 0.0% a 100.0%.
    #[must_use]
    pub fn progress_percent(&self) -> f64 {
        self.progress_to_next_rank() * 100.0
    }
}

/// Evento del diario: `PowerplayMerits` al ganar méritos mediante combate, misiones o comercio.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct PowerplayMerits {
    pub power: String,
    pub merits_earned: u32,
    pub total_merits: u32,
    pub activity: Option<String>,
}

/// Evento del diario: `PowerplayVote` al emitir votos de preparación o consolidación.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct PowerplayVote {
    pub power: String,
    pub votes: u32,
    pub system: String,
}

/// Evento del diario: `PowerplayDefect` al desertar de una potencia a otra.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct PowerplayDefect {
    pub from_power: String,
    pub to_power: String,
}

/// Evento del diario: `PowerplayLeave` al abandonar la potencia jurada.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct PowerplayLeave {
    pub power: String,
}

/// Evento del diario: `PowerplayJoin` al jurar lealtad por primera vez.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct PowerplayJoin {
    pub power: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn powerplay_deserialization_and_helpers() {
        let json = r#"{
            "timestamp":"2026-04-10T12:00:00Z",
            "event":"Powerplay",
            "Power":"Aisling Duval",
            "Rank":4,
            "Merits":9420,
            "Votes":50,
            "TimePledged":8467200
        }"#;

        let pp: Powerplay = serde_json::from_str(json).expect("Powerplay debe deserializar");
        assert_eq!(pp.power, "Aisling Duval");
        assert_eq!(pp.rank, 4);
        assert_eq!(pp.merits, 9420);
        assert_eq!(pp.votes, Some(50));
        // 8.467.200 s / 604.800 s = exactamente 14 semanas
        assert_eq!(pp.weeks_pledged(), 14);

        // Para Rango 4: base = 5.000, target (Rango 5) = 15.000
        assert_eq!(pp.current_rank_base_merits(), 5000);
        assert_eq!(pp.next_rank_target_merits(), 15000);
        // Faltan: 15.000 - 9.420 = 5.580 méritos
        assert_eq!(pp.merits_needed_for_next_rank(), 5580);

        // Progreso dentro del tramo: (9420 - 5000) / (15000 - 5000) = 4420 / 10000 = 44.2 %
        assert_eq!(pp.progress_ratio(), 0.442);
        assert_eq!(pp.progress_percent(), 44.2);
    }

    #[test]
    fn powerplay_merits_and_activity_deserialization() {
        let json = r#"{
            "timestamp":"2026-04-10T12:30:00Z",
            "event":"PowerplayMerits",
            "Power":"Aisling Duval",
            "MeritsEarned":150,
            "TotalMerits":9570,
            "Activity":"Combat"
        }"#;

        let merits: PowerplayMerits =
            serde_json::from_str(json).expect("PowerplayMerits debe deserializar");
        assert_eq!(merits.power, "Aisling Duval");
        assert_eq!(merits.merits_earned, 150);
        assert_eq!(merits.total_merits, 9570);
        assert_eq!(merits.activity.as_deref(), Some("Combat"));
    }

    #[test]
    fn powerplay_vote_and_defect_deserialization() {
        let vote_json = r#"{
            "timestamp":"2026-04-10T13:00:00Z",
            "event":"PowerplayVote",
            "Power":"Aisling Duval",
            "Votes":10,
            "System":"Cubeo"
        }"#;
        let vote: PowerplayVote = serde_json::from_str(vote_json).expect("PowerplayVote");
        assert_eq!(vote.power, "Aisling Duval");
        assert_eq!(vote.votes, 10);
        assert_eq!(vote.system, "Cubeo");

        let defect_json = r#"{
            "timestamp":"2026-04-10T14:00:00Z",
            "event":"PowerplayDefect",
            "FromPower":"Zachary Hudson",
            "ToPower":"Aisling Duval"
        }"#;
        let defect: PowerplayDefect = serde_json::from_str(defect_json).expect("PowerplayDefect");
        assert_eq!(defect.from_power, "Zachary Hudson");
        assert_eq!(defect.to_power, "Aisling Duval");
    }
}
