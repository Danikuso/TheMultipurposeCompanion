//! Tipos de consulta de red compartidos entre módulos, núcleo y capa de red.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::events::MaterialCategory;
use crate::market::PriceOrigin;

/// Consulta de red emitida por un módulo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NetQuery {
    System {
        id64: u64,
        name: String,
    },
    NeutronRoute(NeutronRouteParams),
    GalaxyRoute(GalaxyRouteParams),
    Autocomplete {
        prefix: String,
    },
    /// Capa 3 de exobiología (Canonn). Solo identifica sistema y cuerpo: nunca
    /// lleva datos del jugador.
    BodyBio {
        system_address: u64,
        star_system: String,
        body_id: u32,
        body_name: String,
        profile: BodyProfile,
    },
    /// Estaciones cercanas que importan una materia (Ardent). El filtro de
    /// plataforma se aplica en local sobre el resultado.
    CommodityImportsNearby {
        system_address: u64,
        commodity: String,
        max_distance_ly: u32,
        min_pad: PadSize,
    },
    /// Estación más cercana con un servicio (Ardent; filtra en servidor).
    NearestService {
        system_address: u64,
        /// Nombre del sistema de referencia (Spansh busca por nombre).
        system_name: String,
        service: NearbyService,
        min_pad: PadSize,
    },
    /// Ruta comercial multi-salto (Spansh). Los portaaviones se excluyen siempre.
    TradeRoute(TradeRouteParams),
    /// Estaciones cercanas que exportan una materia (Ardent).
    CommodityExportsNearby {
        system_address: u64,
        commodity: String,
        max_distance_ly: u32,
        min_pad: PadSize,
    },
    /// Coordenadas de un sistema por nombre (no encontrado: `NoData`).
    SystemCoordsByName {
        name: String,
    },
    /// Consulta del último tick galáctico del BGS (EDCD).
    BgsTick,
    /// Búsqueda de astilleros (naves) y equipamiento (módulos) cercanos (Spansh).
    OutfittingSearch(OutfittingSearchParams),
}

/// Tamaño de plataforma de aterrizaje.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PadSize {
    Small,
    Medium,
    Large,
}

impl PadSize {
    /// Convierte el `maxLandingPadSize` de Ardent (1=S, 2=M, 3=L).
    pub fn from_ardent(n: u8) -> Option<Self> {
        match n {
            1 => Some(Self::Small),
            2 => Some(Self::Medium),
            3 => Some(Self::Large),
            _ => None,
        }
    }

    /// `true` si una plataforma máxima `self` admite una nave de tamaño `ship`.
    pub fn fits(self, ship: PadSize) -> bool {
        self >= ship
    }
}

/// Servicio buscable por cercanía (ampliable).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NearbyService {
    InterstellarFactors,
    BlackMarket,
    /// Comerciante de materiales; `None` = cualquier tipo (Ardent), `Some` = tipo concreto (Spansh).
    MaterialTrader(Option<TraderKind>),
    /// Bróker de tecnología; `None` = cualquier tipo (Ardent), `Some` = tipo concreto (Spansh).
    TechnologyBroker(Option<BrokerKind>),
}

/// Tipo de comerciante de materiales.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TraderKind {
    Raw,
    Manufactured,
    Encoded,
}

impl From<MaterialCategory> for TraderKind {
    fn from(c: MaterialCategory) -> Self {
        match c {
            MaterialCategory::Raw => Self::Raw,
            MaterialCategory::Manufactured => Self::Manufactured,
            MaterialCategory::Encoded => Self::Encoded,
        }
    }
}

/// Tipo de bróker de tecnología.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BrokerKind {
    Human,
    Guardian,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NeutronRouteParams {
    pub from: String,
    pub to: String,
    pub range_ly: f64,
    pub efficiency: u8,
    pub supercharge_multiplier: f64,
    #[serde(default)]
    pub from_coords: Option<[f64; 3]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GalaxyRouteParams {
    pub from: String,
    pub to: String,
    pub ship: ShipParams,
    pub refuel_scoopable: bool,
    pub use_injections: bool,
    #[serde(default)]
    pub algorithm: Option<String>,
    #[serde(default)]
    pub from_coords: Option<[f64; 3]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShipParams {
    pub unladen_mass: f64,
    pub fuel_main_capacity: f64,
    pub fuel_reserve: f64,
    pub cargo: f64,
    pub optimal_mass: f64,
    pub max_fuel_per_jump: f64,
    pub fuel_mul: f64,
    pub fuel_power: f64,
    pub guardian_bonus_ly: f64,
    pub supercharge_multiplier: f64,
}

/// Petición con clave de relevancia (`system:<id64>`, `route`, `bio:<addr>:<body>`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetRequest {
    pub id: u64,
    pub module: String,
    pub key: String,
    pub query: NetQuery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetSource {
    Spansh,
    Edsm,
    Canonn,
    Ardent,
    Edcd,
    Eddn,
    Inara,
    Capi,
    Cache,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetResponse {
    pub id: u64,
    pub module: String,
    pub key: String,
    pub source: NetSource,
    pub fetched_at: DateTime<Utc>,
    pub outcome: NetOutcome,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum NetOutcome {
    System(RemoteSystem),
    Route(PlannedRoute),
    Autocomplete(Vec<SystemSuggestion>),
    BodyBio(RemoteBodyBio),
    Markets(Vec<RemoteMarket>),
    Stations(Vec<RemoteStation>),
    TradeRoute(TradeRoutePlan),
    Exports(Vec<RemoteExport>),
    Coords(SystemCoords),
    BgsTick(BgsTickInfo),
    OutfittingStations(OutfittingSearchResult),
    NoData,
    Disabled,
    Failed(NetFailure),
}

impl NetOutcome {
    /// Resumen para diagnóstico: variante y número de elementos, sin contenido.
    pub fn diag_summary(&self) -> (&'static str, usize) {
        match self {
            Self::System(_) => ("system", 1),
            Self::Route(_) => ("route", 1),
            Self::Autocomplete(v) => ("autocomplete", v.len()),
            Self::BodyBio(_) => ("body_bio", 1),
            Self::Markets(v) => ("markets", v.len()),
            Self::Stations(v) => ("stations", v.len()),
            Self::TradeRoute(plan) => ("trade_route", plan.stops.len()),
            Self::Exports(v) => ("exports", v.len()),
            Self::Coords(_) => ("coords", 1),
            Self::BgsTick(_) => ("bgs_tick", 1),
            Self::OutfittingStations(res) => ("outfitting_stations", res.stations.len()),
            Self::NoData => ("no_data", 0),
            Self::Disabled => ("disabled", 0),
            Self::Failed(_) => ("failed", 0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetFailure {
    Timeout,
    Http(u16),
    Malformed,
    Down,
    RateLimited,
    UnknownOrigin,
    UnknownDestination,
}

/// Sugerencia de autocompletado; `id64` lo necesita el planificador de galaxia.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemSuggestion {
    pub name: String,
    pub id64: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteSystem {
    pub id64: u64,
    pub name: String,
    pub discovered: Option<bool>,
    pub body_count: Option<u32>,
    pub primary_star: Option<String>,
    pub allegiance: Option<String>,
    pub government: Option<String>,
    pub controlling_faction: Option<String>,
    pub faction_state: Option<String>,
    pub economy: Option<String>,
    pub second_economy: Option<String>,
    pub security: Option<String>,
    pub population: Option<u64>,
    pub powers: Vec<String>,
    pub powerplay_state: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedRoute {
    pub hops: Vec<PlannedHop>,
    pub total_ly: f64,
}

/// Tramo de ruta. En rutas de neutrones un tramo puede agrupar varios saltos
/// (`jumps`) y `distance_ly` es la distancia del tramo; `refuel` es `None`
/// cuando la fuente no lo informa.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedHop {
    pub name: String,
    pub id64: Option<u64>,
    pub star_pos: Option<[f64; 3]>,
    pub distance_ly: f64,
    pub jumps: u32,
    pub neutron: bool,
    pub refuel: Option<bool>,
    pub star_class: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodyProfile {
    pub planet_class: Option<String>,
    pub atmosphere_type: Option<String>,
    pub gravity_g: Option<f64>,
    pub temperature_k: Option<f64>,
    pub pressure_atm: Option<f64>,
    pub volcanism: Option<String>,
    pub star_class: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteBodyBio {
    pub hints: Vec<RemoteSpeciesHint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteSpeciesHint {
    pub genus: String,
    pub species: String,
    pub weight: f64,
}

/// Mercado remoto que compra una materia (`sell_price` = lo que paga al jugador).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteMarket {
    pub station_name: String,
    pub system_name: String,
    pub market_id: Option<u64>,
    pub distance_ly: f64,
    pub distance_to_arrival_ls: Option<f64>,
    pub sell_price: u64,
    pub demand: Option<u64>,
    pub max_pad: Option<PadSize>,
    pub station_type: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
    /// Procedencia del precio; `None` si el constructor no la informa.
    pub origin: Option<PriceOrigin>,
}

/// Parametros de la ruta comercial (carriers siempre excluidos).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeRouteParams {
    pub system: String,
    pub station: String,
    pub starting_capital: u64,
    pub max_cargo: u32,
    pub max_hops: u8,
    pub max_hop_distance_ly: f64,
    pub min_pad: PadSize,
    pub max_price_age_days: u32,
    pub allow_planetary: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeCommodity {
    pub commodity: String,
    pub name_localised: Option<String>,
    pub amount: u32,
    pub buy_price: u64,
    pub sell_price: u64,
    pub profit_per_ton: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeStop {
    pub from_system: String,
    pub from_station: String,
    pub to_system: String,
    pub to_station: String,
    pub from_market_id: Option<u64>,
    pub to_market_id: Option<u64>,
    pub commodities: Vec<TradeCommodity>,
    pub stop_profit: i64,
    pub distance_ly: f64,
    pub data_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeRoutePlan {
    pub stops: Vec<TradeStop>,
    pub total_profit: i64,
}

/// Mercado remoto que vende (exporta) una materia (`buy_price` = lo que paga el jugador).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteExport {
    pub station_name: String,
    pub system_name: String,
    pub market_id: Option<u64>,
    pub distance_ly: f64,
    pub distance_to_arrival_ls: Option<f64>,
    pub buy_price: u64,
    pub stock: Option<u64>,
    pub max_pad: Option<PadSize>,
    pub station_type: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemCoords {
    pub name: String,
    pub id64: Option<u64>,
    pub star_pos: [f64; 3],
}

/// Información del último tick galáctico del BGS (EDCD).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BgsTickInfo {
    pub latest_tick: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteStation {
    pub station_name: String,
    pub system_name: String,
    pub distance_ly: f64,
    pub distance_to_arrival_ls: Option<f64>,
    pub max_pad: Option<PadSize>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Modo de montaje en armas de anclaje (`weapon_mode` en Spansh).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum WeaponMode {
    Fixed,
    Gimbal,
    Turret,
}

impl WeaponMode {
    pub fn as_spansh_str(self) -> &'static str {
        match self {
            Self::Fixed => "Fixed",
            Self::Gimbal => "Gimbal",
            Self::Turret => "Turret",
        }
    }
}

/// Componente de nave solicitado en una búsqueda de equipamiento.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutfittingModuleQuery {
    pub name: String,
    #[serde(default)]
    pub class: Option<u8>,
    #[serde(default)]
    pub rating: Option<String>,
    #[serde(default)]
    pub weapon_mode: Option<WeaponMode>,
    #[serde(default)]
    pub armour_ship: Option<String>,
}

impl OutfittingModuleQuery {
    /// Etiqueta legible del componente buscado (ej. `5A Frame Shift Drive (SCO)`).
    pub fn display_label(&self) -> String {
        let mut prefix = String::new();
        if let Some(c) = self.class {
            prefix.push_str(&c.to_string());
        }
        if let Some(r) = &self.rating {
            prefix.push_str(r);
        }
        let mut label = if prefix.is_empty() {
            self.name.clone()
        } else {
            format!("{prefix} {}", self.name)
        };
        if let Some(mode) = self.weapon_mode {
            label.push_str(&format!(" ({})", mode.as_spansh_str()));
        }
        if let Some(ship) = &self.armour_ship {
            label.push_str(&format!(" [{ship}]"));
        }
        label
    }
}

/// Parámetros de búsqueda combinada de naves y componentes en estaciones cercanas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutfittingSearchParams {
    pub reference_system: String,
    #[serde(default)]
    pub reference_coords: Option<[f64; 3]>,
    #[serde(default)]
    pub ship: Option<String>,
    #[serde(default)]
    pub modules: Vec<OutfittingModuleQuery>,
    pub min_pad: PadSize,
    #[serde(default)]
    pub only_lyr_discount: bool,
    #[serde(default)]
    pub include_own_stronghold: bool,
    #[serde(default)]
    pub pledged_power: Option<String>,
    #[serde(default)]
    pub include_carriers: bool,
    #[serde(default)]
    pub include_planetary: bool,
}

/// Precio reportado de un elemento solicitado en una estación concreta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockedItemPrice {
    pub label: String,
    pub price_cr: Option<u64>,
}

/// Estación cercana con disponibilidad de la nave y/o componentes solicitados.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutfittingStationHit {
    pub station_name: String,
    pub system_name: String,
    pub station_type: Option<String>,
    pub market_id: Option<u64>,
    pub distance_ly: f64,
    pub distance_to_arrival_ls: Option<f64>,
    pub max_pad: Option<PadSize>,
    pub is_planetary: bool,
    pub is_carrier: bool,
    pub is_stronghold_carrier: bool,
    pub controlling_power: Option<String>,
    pub discount_tag: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
    pub item_prices: Vec<StockedItemPrice>,
    pub total_price_cr: Option<u64>,
}

/// Resultado de respaldo individual por elemento cuando ninguna estación reúne toda la cesta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutfittingFallbackEntry {
    pub item_label: String,
    pub nearest_station: Option<OutfittingStationHit>,
}

/// Resultado completo de una consulta de astilleros y equipamiento.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OutfittingSearchResult {
    pub stations: Vec<OutfittingStationHit>,
    pub fallback: Vec<OutfittingFallbackEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sol() -> RemoteSystem {
        RemoteSystem {
            id64: 1,
            name: "Sol".into(),
            discovered: None,
            body_count: Some(40),
            primary_star: Some("G".into()),
            allegiance: None,
            government: None,
            controlling_faction: None,
            faction_state: None,
            economy: None,
            second_economy: None,
            security: None,
            population: Some(22_780_919_531),
            powers: vec!["Jerome Archer".into()],
            powerplay_state: None,
        }
    }

    fn response(outcome: NetOutcome) -> NetResponse {
        NetResponse {
            id: 3,
            module: "navigation".into(),
            key: "system:1".into(),
            source: NetSource::Spansh,
            fetched_at: "2026-10-17T12:00:00Z".parse().unwrap(),
            outcome,
        }
    }

    #[test]
    fn net_response_roundtrips_json() {
        for x in [
            response(NetOutcome::System(sol())),
            response(NetOutcome::Failed(NetFailure::Http(503))),
            response(NetOutcome::Route(PlannedRoute {
                hops: vec![PlannedHop {
                    name: "Colonia".into(),
                    id64: Some(7),
                    star_pos: Some([1.0, 2.0, 3.0]),
                    distance_ly: 12.5,
                    jumps: 2,
                    neutron: true,
                    refuel: None,
                    star_class: Some("N".into()),
                }],
                total_ly: 12.5,
            })),
        ] {
            let text = serde_json::to_string(&x).unwrap();
            let back: NetResponse = serde_json::from_str(&text).unwrap();
            assert_eq!(back, x);
        }
        assert_eq!(response(NetOutcome::NoData).id, 3);
    }

    #[test]
    fn commodity_imports_query_roundtrips_json() {
        let q = NetQuery::CommodityImportsNearby {
            system_address: 10477373803,
            commodity: "platinum".into(),
            max_distance_ly: 100,
            min_pad: PadSize::Large,
        };
        let back: NetQuery = serde_json::from_str(&serde_json::to_string(&q).unwrap()).unwrap();
        assert_eq!(back, q);
        let n = NetQuery::NearestService {
            system_address: 1,
            system_name: "Sol".into(),
            service: NearbyService::InterstellarFactors,
            min_pad: PadSize::Medium,
        };
        let back: NetQuery = serde_json::from_str(&serde_json::to_string(&n).unwrap()).unwrap();
        assert_eq!(back, n);
    }

    #[test]
    fn markets_outcome_roundtrips_with_ardent_source() {
        let mut r = response(NetOutcome::Markets(vec![RemoteMarket {
            station_name: "Jameson Memorial".into(),
            system_name: "Shinrarta Dezhra".into(),
            market_id: Some(128666762),
            distance_ly: 12.0,
            distance_to_arrival_ls: Some(450.5),
            sell_price: 220000,
            demand: Some(300),
            max_pad: Some(PadSize::Medium),
            station_type: None,
            updated_at: Some("2026-10-17T12:00:00Z".parse().unwrap()),
            origin: Some(PriceOrigin::Ardent),
        }]));
        r.source = NetSource::Ardent;
        let back: NetResponse = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(back, r);
        assert_eq!(back.source, NetSource::Ardent);
        let st = response(NetOutcome::Stations(vec![RemoteStation {
            station_name: "X".into(),
            system_name: "Y".into(),
            distance_ly: 3.0,
            distance_to_arrival_ls: None,
            max_pad: Some(PadSize::Small),
            updated_at: None,
        }]));
        let back: NetResponse = serde_json::from_str(&serde_json::to_string(&st).unwrap()).unwrap();
        assert_eq!(back, st);
    }

    #[test]
    fn trader_kind_from_category() {
        assert_eq!(
            TraderKind::from(MaterialCategory::Encoded),
            TraderKind::Encoded
        );
        assert_eq!(TraderKind::from(MaterialCategory::Raw), TraderKind::Raw);
        let q = NetQuery::NearestService {
            system_address: 1,
            system_name: "Sol".into(),
            service: NearbyService::MaterialTrader(Some(TraderKind::Raw)),
            min_pad: PadSize::Small,
        };
        let back: NetQuery = serde_json::from_str(&serde_json::to_string(&q).unwrap()).unwrap();
        assert_eq!(back, q);
    }

    #[test]
    fn pad_size_fits() {
        assert!(PadSize::Large.fits(PadSize::Medium));
        assert!(!PadSize::Medium.fits(PadSize::Large));
        assert!(PadSize::Small.fits(PadSize::Small));
        assert_eq!(PadSize::from_ardent(3), Some(PadSize::Large));
        assert_eq!(PadSize::from_ardent(1), Some(PadSize::Small));
        assert_eq!(PadSize::from_ardent(9), None);
    }

    #[test]
    fn net_failure_http_serializes_stably() {
        let text = serde_json::to_string(&NetFailure::Http(503)).unwrap();
        assert_eq!(text, r#"{"Http":503}"#);
        assert_eq!(
            serde_json::from_str::<NetFailure>(&text).unwrap(),
            NetFailure::Http(503)
        );
    }

    #[test]
    fn trade_route_query_roundtrips() {
        let q = NetQuery::TradeRoute(TradeRouteParams {
            system: "Lave".into(),
            station: "Lave Station".into(),
            starting_capital: 5_000_000,
            max_cargo: 256,
            max_hops: 5,
            max_hop_distance_ly: 30.0,
            min_pad: PadSize::Large,
            max_price_age_days: 2,
            allow_planetary: true,
        });
        let back: NetQuery = serde_json::from_str(&serde_json::to_string(&q).unwrap()).unwrap();
        assert_eq!(back, q);
        let e = NetQuery::CommodityExportsNearby {
            system_address: 1,
            commodity: "gold".into(),
            max_distance_ly: 50,
            min_pad: PadSize::Small,
        };
        let back: NetQuery = serde_json::from_str(&serde_json::to_string(&e).unwrap()).unwrap();
        assert_eq!(back, e);
        let c = NetQuery::SystemCoordsByName { name: "Sol".into() };
        let back: NetQuery = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn trade_route_outcome_roundtrips() {
        let stop = |from: &str, to: &str| TradeStop {
            from_system: from.into(),
            from_station: format!("{from} Station"),
            to_system: to.into(),
            to_station: format!("{to} Station"),
            from_market_id: Some(1),
            to_market_id: None,
            commodities: vec![TradeCommodity {
                commodity: "gold".into(),
                name_localised: Some("Oro".into()),
                amount: 256,
                buy_price: 9_000,
                sell_price: 15_000,
                profit_per_ton: 6_000,
            }],
            stop_profit: 1_600_000,
            distance_ly: 12.5,
            data_updated_at: Some("2026-10-17T12:00:00Z".parse().unwrap()),
        };
        let r = response(NetOutcome::TradeRoute(TradeRoutePlan {
            stops: vec![stop("Lave", "Leesti"), stop("Leesti", "Lave")],
            total_profit: 3_200_000,
        }));
        let back: NetResponse = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(back, r);
        let ex = response(NetOutcome::Exports(vec![RemoteExport {
            station_name: "A".into(),
            system_name: "B".into(),
            market_id: None,
            distance_ly: 1.0,
            distance_to_arrival_ls: None,
            buy_price: 10,
            stock: Some(5),
            max_pad: Some(PadSize::Large),
            station_type: None,
            updated_at: None,
        }]));
        let back: NetResponse = serde_json::from_str(&serde_json::to_string(&ex).unwrap()).unwrap();
        assert_eq!(back, ex);
    }

    #[test]
    fn coords_outcome_roundtrips() {
        let r = response(NetOutcome::Coords(SystemCoords {
            name: "Sol".into(),
            id64: Some(10477373803),
            star_pos: [0.0, 0.0, 0.0],
        }));
        let back: NetResponse = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn outfitting_search_query_and_outcome_roundtrip_and_summarize() {
        let params = OutfittingSearchParams {
            reference_system: "Sol".into(),
            reference_coords: Some([0.0, 0.0, 0.0]),
            ship: Some("Mandalay".into()),
            modules: vec![
                OutfittingModuleQuery {
                    name: "Frame Shift Drive (SCO)".into(),
                    class: Some(5),
                    rating: Some("A".into()),
                    weapon_mode: None,
                    armour_ship: None,
                },
                OutfittingModuleQuery {
                    name: "Multi-Cannon".into(),
                    class: Some(3),
                    rating: Some("C".into()),
                    weapon_mode: Some(WeaponMode::Gimbal),
                    armour_ship: None,
                },
            ],
            min_pad: PadSize::Medium,
            only_lyr_discount: true,
            include_own_stronghold: true,
            pledged_power: Some("Li Yong-Rui".into()),
            include_carriers: false,
            include_planetary: true,
        };
        let q = NetQuery::OutfittingSearch(params);
        let back_q: NetQuery = serde_json::from_str(&serde_json::to_string(&q).unwrap()).unwrap();
        assert_eq!(back_q, q);

        let hit = OutfittingStationHit {
            station_name: "Ray Gateway".into(),
            system_name: "Diaguandri".into(),
            station_type: Some("Coriolis Starport".into()),
            market_id: Some(128066403),
            distance_ly: 123.4,
            distance_to_arrival_ls: Some(392.0),
            max_pad: Some(PadSize::Large),
            is_planetary: false,
            is_carrier: false,
            is_stronghold_carrier: false,
            controlling_power: Some("Li Yong-Rui".into()),
            discount_tag: Some("lyr_15".into()),
            updated_at: Some("2026-10-28T12:00:00Z".parse().unwrap()),
            item_prices: vec![
                StockedItemPrice {
                    label: "Mandalay".into(),
                    price_cr: Some(14_993_338),
                },
                StockedItemPrice {
                    label: "5A Frame Shift Drive (SCO)".into(),
                    price_cr: Some(5_206_031),
                },
            ],
            total_price_cr: Some(20_199_369),
        };
        let outcome = NetOutcome::OutfittingStations(OutfittingSearchResult {
            stations: vec![hit.clone()],
            fallback: vec![OutfittingFallbackEntry {
                item_label: "Mandalay".into(),
                nearest_station: Some(hit),
            }],
        });
        assert_eq!(outcome.diag_summary(), ("outfitting_stations", 1));
        let r = response(outcome);
        let back_r: NetResponse =
            serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(back_r, r);
    }
}
