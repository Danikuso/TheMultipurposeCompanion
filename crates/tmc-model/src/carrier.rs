//! Modelos y eventos tipados para la gestión de Fleet Carriers (portaaviones).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Semáforo de salud financiera y semanas de mantenimiento cubiertas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaintenanceHealth {
    /// Seguro: más de 4 semanas de mantenimiento cubiertas en el saldo disponible.
    Safe,
    /// Advertencia: entre 2 y 4 semanas de mantenimiento cubiertas.
    Warning,
    /// Crítico: menos de 2 semanas de mantenimiento (riesgo inminente de deuda o embargo).
    Critical,
}

/// Uso de espacio y bodega del Fleet Carrier (capacidad nominal 25.000 t).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct CarrierSpaceUsage {
    pub total_capacity: u32,
    pub crew: u32,
    pub cargo: u32,
    pub cargo_space_reserved: u32,
    pub ship_packs: u32,
    pub module_packs: u32,
    pub free_space: u32,
}

/// Estado financiero del carrier: saldos, reserva y tasas arancelarias de servicios.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct CarrierFinanceInfo {
    pub carrier_balance: i64,
    pub reserve_balance: i64,
    pub available_balance: i64,
    pub reserve_percent: u8,
    #[serde(rename = "TaxRate_rearm")]
    pub tax_rate_rearm: Option<u8>,
    #[serde(rename = "TaxRate_refuel")]
    pub tax_rate_refuel: Option<u8>,
    #[serde(rename = "TaxRate_repair")]
    pub tax_rate_repair: Option<u8>,
}

impl CarrierFinanceInfo {
    /// Calcula las semanas de mantenimiento que cubre el saldo disponible dado un coste semanal estimado.
    #[must_use]
    pub fn weeks_covered(&self, weekly_upkeep: i64) -> f64 {
        if weekly_upkeep <= 0 {
            return 999.0;
        }
        if self.carrier_balance <= 0 {
            return 0.0;
        }
        self.carrier_balance as f64 / weekly_upkeep as f64
    }

    /// Evalúa el semáforo de salud según las semanas cubiertas:
    /// - > 4.0 semanas: `Safe`
    /// - 2.0 ..= 4.0 semanas: `Warning`
    /// - < 2.0 semanas: `Critical`
    #[must_use]
    pub fn health(&self, weekly_upkeep: i64) -> MaintenanceHealth {
        let weeks = self.weeks_covered(weekly_upkeep);
        if weeks > 4.0 {
            MaintenanceHealth::Safe
        } else if weeks >= 2.0 {
            MaintenanceHealth::Warning
        } else {
            MaintenanceHealth::Critical
        }
    }
}

/// Miembro o servicio de tripulación a bordo del carrier.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct CarrierCrewMember {
    pub crew_role: String,
    pub activated: bool,
    pub enabled: bool,
    pub crew_name: Option<String>,
}

/// Paquete comercial de naves o módulos instalado en el astillero / equipamiento del carrier.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct CarrierPack {
    pub pack_theme: String,
    pub pack_tier: u32,
    pub cost: Option<u64>,
}

/// Evento del diario: `CarrierStats` con el estado completo del portaaviones.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct CarrierStats {
    #[serde(rename = "CarrierID")]
    pub carrier_id: u64,
    pub callsign: String,
    pub name: String,
    pub docking_access: String,
    pub allow_notorious: bool,
    pub fuel_level: u32,
    pub jump_range_curr: f64,
    pub jump_range_max: f64,
    pub pending_decommission: bool,
    pub space_usage: CarrierSpaceUsage,
    pub finance: CarrierFinanceInfo,
    pub crew: Vec<CarrierCrewMember>,
    pub ship_packs: Vec<CarrierPack>,
    pub module_packs: Vec<CarrierPack>,
}

impl Default for CarrierStats {
    fn default() -> Self {
        Self {
            carrier_id: 0,
            callsign: String::new(),
            name: String::new(),
            docking_access: "all".to_string(),
            allow_notorious: false,
            fuel_level: 0,
            jump_range_curr: 500.0,
            jump_range_max: 500.0,
            pending_decommission: false,
            space_usage: CarrierSpaceUsage::default(),
            finance: CarrierFinanceInfo::default(),
            crew: Vec::new(),
            ship_packs: Vec::new(),
            module_packs: Vec::new(),
        }
    }
}

/// Evento del diario: `CarrierJumpRequest` al programar un salto interestelar.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct CarrierJumpRequest {
    #[serde(rename = "CarrierID")]
    pub carrier_id: u64,
    pub system_name: String,
    pub body: Option<String>,
    #[serde(rename = "BodyID")]
    pub body_id: Option<u32>,
    pub system_address: Option<u64>,
    pub departure_time: Option<DateTime<Utc>>,
}

/// Evento del diario: `CarrierJumpCancelled` cuando se aborta un salto programado.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct CarrierJumpCancelled {
    #[serde(rename = "CarrierID")]
    pub carrier_id: u64,
}

/// Evento del diario: `CarrierDepositFuel` al repostar tritio en el depósito del carrier.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct CarrierDepositFuel {
    #[serde(rename = "CarrierID")]
    pub carrier_id: u64,
    pub amount: u32,
    pub total: u32,
}

/// Evento del diario: `CarrierFinance` actualización periódica o manual de fondos y aranceles.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct CarrierFinance {
    #[serde(rename = "CarrierID")]
    pub carrier_id: u64,
    pub carrier_balance: i64,
    pub reserve_balance: i64,
    pub available_balance: i64,
    pub reserve_percent: u8,
    #[serde(rename = "TaxRate_rearm")]
    pub tax_rate_rearm: Option<u8>,
    #[serde(rename = "TaxRate_refuel")]
    pub tax_rate_refuel: Option<u8>,
    #[serde(rename = "TaxRate_repair")]
    pub tax_rate_repair: Option<u8>,
}

/// Evento del diario: `CarrierDecommission` cuando se programa la venta/desguace del carrier.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct CarrierDecommission {
    #[serde(rename = "CarrierID")]
    pub carrier_id: u64,
    pub scrap_refund: i64,
    pub scrap_time: i64,
}

/// Evento del diario: `CarrierBankTransfer` transferencias entre saldo del comandante y el carrier.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct CarrierBankTransfer {
    #[serde(rename = "CarrierID")]
    pub carrier_id: u64,
    pub deposit: Option<i64>,
    pub withdraw: Option<i64>,
    pub carrier_balance: i64,
    pub player_balance: i64,
}

/// Evento del diario: `CarrierCrewServices` activación, sustitución o pausa de servicios de tripulación.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct CarrierCrewServices {
    #[serde(rename = "CarrierID")]
    pub carrier_id: u64,
    pub crew_role: String,
    pub operation: String,
    pub crew_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carrier_stats_deserialization() {
        let json = r#"{
            "timestamp":"2020-03-09T11:42:01Z",
            "event":"CarrierStats",
            "CarrierID":3700000000,
            "Callsign":"ABC-123",
            "Name":"STARGAZER",
            "DockingAccess":"all",
            "AllowNotorious":false,
            "FuelLevel":750,
            "JumpRangeCurr":500.0,
            "JumpRangeMax":500.0,
            "PendingDecommission":false,
            "SpaceUsage":{
                "TotalCapacity":25000,
                "Crew":100,
                "Cargo":10000,
                "CargoSpaceReserved":500,
                "ShipPacks":100,
                "ModulePacks":100,
                "FreeSpace":14200
            },
            "Finance":{
                "CarrierBalance":1000000000,
                "ReserveBalance":100000000,
                "AvailableBalance":900000000,
                "ReservePercent":10,
                "TaxRate_rearm":5,
                "TaxRate_refuel":5,
                "TaxRate_repair":5
            },
            "Crew":[
                { "CrewRole":"Refuel", "Activated":true, "Enabled":true, "CrewName":"Alba Rossi" }
            ],
            "ShipPacks":[],
            "ModulePacks":[]
        }"#;

        let stats: CarrierStats =
            serde_json::from_str(json).expect("CarrierStats debe deserializar");
        assert_eq!(stats.carrier_id, 3700000000);
        assert_eq!(stats.callsign, "ABC-123");
        assert_eq!(stats.name, "STARGAZER");
        assert_eq!(stats.fuel_level, 750);
        assert_eq!(stats.jump_range_curr, 500.0);
        assert_eq!(stats.space_usage.free_space, 14200);
        assert_eq!(stats.finance.carrier_balance, 1000000000);
        assert_eq!(stats.crew.len(), 1);
        assert_eq!(stats.crew[0].crew_name.as_deref(), Some("Alba Rossi"));

        // Prueba de cobertura de mantenimiento: coste semanal 25M
        assert_eq!(stats.finance.weeks_covered(25_000_000), 40.0);
        assert_eq!(stats.finance.health(25_000_000), MaintenanceHealth::Safe);
    }

    #[test]
    fn carrier_finance_health_levels() {
        let mut fin = CarrierFinanceInfo {
            carrier_balance: 100_000_000,
            reserve_balance: 0,
            available_balance: 100_000_000,
            reserve_percent: 0,
            tax_rate_rearm: None,
            tax_rate_refuel: None,
            tax_rate_repair: None,
        };

        let weekly_upkeep = 20_000_000;
        // 100M / 20M = 5 semanas -> Safe (> 4)
        assert_eq!(fin.health(weekly_upkeep), MaintenanceHealth::Safe);

        // 60M / 20M = 3 semanas -> Warning (2..=4)
        fin.carrier_balance = 60_000_000;
        assert_eq!(fin.health(weekly_upkeep), MaintenanceHealth::Warning);

        // 30M / 20M = 1.5 semanas -> Critical (< 2)
        fin.carrier_balance = 30_000_000;
        assert_eq!(fin.health(weekly_upkeep), MaintenanceHealth::Critical);
    }

    #[test]
    fn carrier_jump_request_and_cancel_deserialization() {
        let req_json = r#"{
            "timestamp":"2020-03-09T12:00:00Z",
            "event":"CarrierJumpRequest",
            "CarrierID":3700000000,
            "SystemName":"Sol",
            "Body":"Earth",
            "BodyID":3,
            "SystemAddress":2869502930266,
            "DepartureTime":"2020-03-09T12:15:00Z"
        }"#;

        let req: CarrierJumpRequest =
            serde_json::from_str(req_json).expect("CarrierJumpRequest debe deserializar");
        assert_eq!(req.carrier_id, 3700000000);
        assert_eq!(req.system_name, "Sol");
        assert_eq!(req.body.as_deref(), Some("Earth"));
        assert_eq!(req.body_id, Some(3));
        assert!(req.departure_time.is_some());

        let cancel_json = r#"{
            "timestamp":"2020-03-09T12:05:00Z",
            "event":"CarrierJumpCancelled",
            "CarrierID":3700000000
        }"#;

        let cancel: CarrierJumpCancelled =
            serde_json::from_str(cancel_json).expect("CarrierJumpCancelled debe deserializar");
        assert_eq!(cancel.carrier_id, 3700000000);
    }

    #[test]
    fn carrier_deposit_fuel_deserialization() {
        let json = r#"{
            "timestamp":"2020-03-09T12:30:00Z",
            "event":"CarrierDepositFuel",
            "CarrierID":3700000000,
            "Amount":250,
            "Total":1000
        }"#;

        let fuel: CarrierDepositFuel =
            serde_json::from_str(json).expect("CarrierDepositFuel debe deserializar");
        assert_eq!(fuel.carrier_id, 3700000000);
        assert_eq!(fuel.amount, 250);
        assert_eq!(fuel.total, 1000);
    }

    #[test]
    fn carrier_bank_transfer_deserialization() {
        let json = r#"{
            "timestamp":"2020-03-09T12:45:00Z",
            "event":"CarrierBankTransfer",
            "CarrierID":3700000000,
            "Deposit":50000000,
            "CarrierBalance":1050000000,
            "PlayerBalance":450000000
        }"#;

        let transfer: CarrierBankTransfer =
            serde_json::from_str(json).expect("CarrierBankTransfer debe deserializar");
        assert_eq!(transfer.deposit, Some(50000000));
        assert_eq!(transfer.withdraw, None);
        assert_eq!(transfer.carrier_balance, 1050000000);
        assert_eq!(transfer.player_balance, 450000000);
    }
}
