//! Modelos y eventos para la Colonización de Sistemas estelares (Trailblazers / Depósitos de obra).

use serde::{Deserialize, Serialize};

/// Recurso o material individual requerido para la construcción de una colonia o estación.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct ColonisationResource {
    pub name: String,
    #[serde(rename = "Name_Localised")]
    pub name_localised: Option<String>,
    pub required_amount: u32,
    pub provided_amount: u32,
}

impl ColonisationResource {
    /// Cantidad que aún falta entregar de este recurso.
    #[must_use]
    pub fn remaining(&self) -> u32 {
        self.required_amount.saturating_sub(self.provided_amount)
    }

    /// Ratio de progreso de este recurso (0.0 a 1.0).
    #[must_use]
    pub fn progress_ratio(&self) -> f64 {
        if self.required_amount == 0 {
            return 1.0;
        }
        (self.provided_amount as f64 / self.required_amount as f64).clamp(0.0, 1.0)
    }

    /// Porcentaje de progreso de 0 a 100.
    #[must_use]
    pub fn progress_percent(&self) -> f64 {
        self.progress_ratio() * 100.0
    }

    /// Viajes restantes con una bodega de capacidad dada.
    #[must_use]
    pub fn trips_remaining(&self, cargo_capacity: u32) -> u32 {
        let rem = self.remaining();
        if rem == 0 || cargo_capacity == 0 {
            return 0;
        }
        rem.div_ceil(cargo_capacity)
    }
}

/// Evento del diario: `ColonisationConstructionDepot` al consultar un depósito de obra en construcción.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct ColonisationConstructionDepot {
    #[serde(rename = "MarketID")]
    pub market_id: Option<u64>,
    pub station_name: Option<String>,
    pub construction_type: Option<String>,
    pub construction_progress: f64,
    pub resources_required: Vec<ColonisationResource>,
}

impl ColonisationConstructionDepot {
    /// Suma total de toneladas que faltan por entregar en todo el proyecto.
    #[must_use]
    pub fn total_remaining_tonnes(&self) -> u32 {
        self.resources_required
            .iter()
            .map(ColonisationResource::remaining)
            .sum()
    }

    /// Suma total de toneladas requeridas en todo el proyecto.
    #[must_use]
    pub fn total_required_tonnes(&self) -> u32 {
        self.resources_required
            .iter()
            .map(|r| r.required_amount)
            .sum()
    }

    /// Suma total de toneladas ya provistas.
    #[must_use]
    pub fn total_provided_tonnes(&self) -> u32 {
        self.resources_required
            .iter()
            .map(|r| r.provided_amount)
            .sum()
    }

    /// Estimación de viajes totales restantes necesarios para completar todos los recursos.
    #[must_use]
    pub fn total_trips_remaining(&self, cargo_capacity: u32) -> u32 {
        let total_rem = self.total_remaining_tonnes();
        if total_rem == 0 || cargo_capacity == 0 {
            return 0;
        }
        total_rem.div_ceil(cargo_capacity)
    }
}

/// Evento del diario: `ColonisationContribution` al entregar materiales en el depósito.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct ColonisationContribution {
    #[serde(rename = "MarketID")]
    pub market_id: Option<u64>,
    pub commodity: String,
    #[serde(rename = "Commodity_Localised")]
    pub commodity_localised: Option<String>,
    pub amount: u32,
    pub remaining_amount: Option<u32>,
    pub reward_credits: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colonisation_depot_deserialization_and_helpers() {
        let json = r#"{
            "timestamp":"2026-04-10T15:00:00Z",
            "event":"ColonisationConstructionDepot",
            "MarketID":3800000000,
            "StationName":"Trailblazer Outpost Alpha",
            "ConstructionType":"PlanetaryOutpost",
            "ConstructionProgress":0.625,
            "ResourcesRequired":[
                { "Name":"titanium", "Name_Localised":"Titanio", "RequiredAmount":5000, "ProvidedAmount":3125 },
                { "Name":"steel", "Name_Localised":"Acero", "RequiredAmount":3000, "ProvidedAmount":1875 }
            ]
        }"#;

        let depot: ColonisationConstructionDepot =
            serde_json::from_str(json).expect("depot debe deserializar");
        assert_eq!(depot.market_id, Some(3800000000));
        assert_eq!(
            depot.station_name.as_deref(),
            Some("Trailblazer Outpost Alpha")
        );
        assert_eq!(depot.construction_progress, 0.625);
        assert_eq!(depot.resources_required.len(), 2);

        let titanium = &depot.resources_required[0];
        assert_eq!(titanium.name, "titanium");
        assert_eq!(titanium.remaining(), 1875);
        assert_eq!(titanium.progress_ratio(), 0.625);
        assert_eq!(titanium.progress_percent(), 62.5);

        // Con una Cutter de 720 t de bodega: ceil(1875 / 720) = 3 viajes
        assert_eq!(titanium.trips_remaining(720), 3);

        let steel = &depot.resources_required[1];
        assert_eq!(steel.remaining(), 1125);
        // ceil(1125 / 720) = 2 viajes
        assert_eq!(steel.trips_remaining(720), 2);

        // Total restante: 1875 + 1125 = 3000 t
        assert_eq!(depot.total_remaining_tonnes(), 3000);
        // Viajes totales: ceil(3000 / 720) = 5 viajes
        assert_eq!(depot.total_trips_remaining(720), 5);
    }

    #[test]
    fn colonisation_contribution_deserialization() {
        let json = r#"{
            "timestamp":"2026-04-10T15:30:00Z",
            "event":"ColonisationContribution",
            "MarketID":3800000000,
            "Commodity":"titanium",
            "Commodity_Localised":"Titanio",
            "Amount":720,
            "RemainingAmount":1155,
            "RewardCredits":1440000
        }"#;

        let contrib: ColonisationContribution =
            serde_json::from_str(json).expect("contrib debe deserializar");
        assert_eq!(contrib.market_id, Some(3800000000));
        assert_eq!(contrib.commodity, "titanium");
        assert_eq!(contrib.amount, 720);
        assert_eq!(contrib.remaining_amount, Some(1155));
        assert_eq!(contrib.reward_credits, Some(1440000));
    }
}
