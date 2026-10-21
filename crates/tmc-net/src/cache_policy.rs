//! Política de caducidad de la caché y claves por consulta.

use crate::ardent;
use std::time::Duration;
use tmc_model::net::{BrokerKind, NearbyService, NetQuery, PadSize, TraderKind};

const HOUR: u64 = 3600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TtlPolicy {
    /// Mientras dure, se sirve sin refrescar.
    pub fresh: Duration,
    /// Pasado `fresh` y hasta aquí, se sirve con antigüedad y se refresca en segundo plano.
    pub max_stale: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CacheKind {
    SystemCard,
    SystemPolitics,
    BodyBio,
    ArdentImports,
    ArdentNearest,
    SpanshTrade,
    ArdentExports,
    SystemCoords,
}

pub fn ttl_for(kind: CacheKind) -> TtlPolicy {
    let secs = |h: u64| Duration::from_secs(h * HOUR);
    match kind {
        CacheKind::SystemCard => TtlPolicy {
            fresh: secs(24),
            max_stale: secs(24),
        },
        CacheKind::SystemPolitics => TtlPolicy {
            fresh: secs(1),
            max_stale: secs(24),
        },
        CacheKind::BodyBio => TtlPolicy {
            fresh: secs(30 * 24),
            max_stale: secs(30 * 24),
        },
        // Mercados cambian deprisa: 15 min, sin servir caducado.
        CacheKind::ArdentImports => TtlPolicy {
            fresh: Duration::from_secs(15 * 60),
            max_stale: Duration::from_secs(15 * 60),
        },
        CacheKind::ArdentNearest => TtlPolicy {
            fresh: secs(1),
            max_stale: secs(1),
        },
        CacheKind::SpanshTrade | CacheKind::ArdentExports => TtlPolicy {
            fresh: Duration::from_secs(15 * 60),
            max_stale: Duration::from_secs(15 * 60),
        },
        // Las coordenadas de un sistema no cambian.
        CacheKind::SystemCoords => TtlPolicy {
            fresh: secs(30 * 24),
            max_stale: secs(30 * 24),
        },
    }
}

fn pad_tag(p: PadSize) -> &'static str {
    match p {
        PadSize::Small => "s",
        PadSize::Medium => "m",
        PadSize::Large => "l",
    }
}

fn trader_tag(k: TraderKind) -> &'static str {
    match k {
        TraderKind::Raw => "raw",
        TraderKind::Manufactured => "manufactured",
        TraderKind::Encoded => "encoded",
    }
}

fn broker_tag(k: BrokerKind) -> &'static str {
    match k {
        BrokerKind::Human => "human",
        BrokerKind::Guardian => "guardian",
    }
}

/// Hash FNV-1a estable (la clave se guarda en disco: no sirve `DefaultHasher`).
fn fnv1a(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3)
    })
}

/// Claves de caché de una consulta; vacío si no se cachea (rutas, autocompletado).
pub fn cache_keys(q: &NetQuery) -> Vec<(CacheKind, String)> {
    match q {
        NetQuery::System { id64, .. } => vec![
            (CacheKind::SystemCard, format!("spansh:system:{id64}:card")),
            (
                CacheKind::SystemPolitics,
                format!("spansh:system:{id64}:politics"),
            ),
        ],
        NetQuery::BodyBio {
            system_address,
            body_id,
            ..
        } => vec![(
            CacheKind::BodyBio,
            format!("canonn:body:{system_address}:{body_id}"),
        )],
        NetQuery::NeutronRoute(_) | NetQuery::GalaxyRoute(_) | NetQuery::Autocomplete { .. } => {
            Vec::new()
        }
        NetQuery::TradeRoute(p) => vec![(
            CacheKind::SpanshTrade,
            format!("spansh:trade:{:016x}", fnv1a(&format!("{p:?}"))),
        )],
        NetQuery::CommodityExportsNearby {
            system_address,
            commodity,
            max_distance_ly,
            min_pad,
        } => vec![(
            CacheKind::ArdentExports,
            format!(
                "ardent:exports:{system_address}:{}:{max_distance_ly}:{}",
                ardent::ardent_commodity(commodity),
                pad_tag(*min_pad)
            ),
        )],
        NetQuery::SystemCoordsByName { name } => vec![(
            CacheKind::SystemCoords,
            format!("coords:{}", name.trim().to_lowercase()),
        )],
        NetQuery::CommodityImportsNearby {
            system_address,
            commodity,
            max_distance_ly,
            min_pad,
        } => vec![(
            CacheKind::ArdentImports,
            format!(
                "ardent:imports:{system_address}:{}:{max_distance_ly}:{}",
                ardent::ardent_commodity(commodity),
                pad_tag(*min_pad)
            ),
        )],
        NetQuery::NearestService {
            system_address,
            service,
            min_pad,
            ..
        } => match service {
            NearbyService::InterstellarFactors => vec![(
                CacheKind::ArdentNearest,
                format!(
                    "ardent:nearest:{system_address}:interstellar-factors:{}",
                    pad_tag(*min_pad)
                ),
            )],
            // Sin tipo: Ardent. Con tipo: Spansh (la clave incluye servicio, tipo y plataforma).
            // Todos comparten la política de 1 h de `ArdentNearest`.
            NearbyService::MaterialTrader(None) => vec![(
                CacheKind::ArdentNearest,
                format!(
                    "ardent:nearest:{system_address}:material-trader:{}",
                    pad_tag(*min_pad)
                ),
            )],
            NearbyService::TechnologyBroker(None) => vec![(
                CacheKind::ArdentNearest,
                format!(
                    "ardent:nearest:{system_address}:technology-broker:{}",
                    pad_tag(*min_pad)
                ),
            )],
            NearbyService::MaterialTrader(Some(k)) => vec![(
                CacheKind::ArdentNearest,
                format!(
                    "spansh:stations:{system_address}:material-trader:{}:{}",
                    trader_tag(*k),
                    pad_tag(*min_pad)
                ),
            )],
            NearbyService::TechnologyBroker(Some(b)) => vec![(
                CacheKind::ArdentNearest,
                format!(
                    "spansh:stations:{system_address}:technology-broker:{}:{}",
                    broker_tag(*b),
                    pad_tag(*min_pad)
                ),
            )],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tmc_model::net::{
        BrokerKind, NearbyService, NetQuery, NeutronRouteParams, PadSize, TraderKind,
    };

    const H: u64 = 3600;

    #[test]
    fn system_card_ttl_24h_and_politics_1h() {
        let c = ttl_for(CacheKind::SystemCard);
        assert_eq!(c.fresh, Duration::from_secs(24 * H));
        assert_eq!(c.max_stale, Duration::from_secs(24 * H));
        let p = ttl_for(CacheKind::SystemPolitics);
        assert_eq!(p.fresh, Duration::from_secs(H));
        assert_eq!(p.max_stale, Duration::from_secs(24 * H));
        let b = ttl_for(CacheKind::BodyBio);
        assert_eq!(b.fresh, Duration::from_secs(30 * 24 * H));
        assert_eq!(b.max_stale, Duration::from_secs(30 * 24 * H));
    }

    #[test]
    fn ardent_ttls_and_keys() {
        let i = ttl_for(CacheKind::ArdentImports);
        assert_eq!(i.fresh, Duration::from_secs(15 * 60));
        assert_eq!(i.max_stale, Duration::from_secs(15 * 60));
        let n = ttl_for(CacheKind::ArdentNearest);
        assert_eq!(n.fresh, Duration::from_secs(H));
        assert_eq!(n.max_stale, Duration::from_secs(H));
        assert_eq!(
            cache_keys(&NetQuery::CommodityImportsNearby {
                system_address: 10477373803,
                commodity: "Platinum".into(),
                max_distance_ly: 100,
                min_pad: PadSize::Large,
            }),
            vec![(
                CacheKind::ArdentImports,
                "ardent:imports:10477373803:platinum:100:l".to_string()
            )]
        );
        assert_eq!(
            cache_keys(&NetQuery::NearestService {
                system_address: 7,
                system_name: "Sol".into(),
                service: NearbyService::InterstellarFactors,
                min_pad: PadSize::Medium,
            }),
            vec![(
                CacheKind::ArdentNearest,
                "ardent:nearest:7:interstellar-factors:m".to_string()
            )]
        );
    }

    #[test]
    fn cache_key_differs_per_service_kind_and_pad() {
        let key = |service, min_pad| {
            cache_keys(&NetQuery::NearestService {
                system_address: 7,
                system_name: "Sol".into(),
                service,
                min_pad,
            })
        };
        let all = [
            key(
                NearbyService::MaterialTrader(Some(TraderKind::Raw)),
                PadSize::Medium,
            ),
            key(
                NearbyService::MaterialTrader(Some(TraderKind::Encoded)),
                PadSize::Medium,
            ),
            key(
                NearbyService::MaterialTrader(Some(TraderKind::Raw)),
                PadSize::Large,
            ),
            key(NearbyService::MaterialTrader(None), PadSize::Medium),
            key(NearbyService::TechnologyBroker(None), PadSize::Medium),
            key(
                NearbyService::TechnologyBroker(Some(BrokerKind::Guardian)),
                PadSize::Medium,
            ),
            key(
                NearbyService::TechnologyBroker(Some(BrokerKind::Human)),
                PadSize::Medium,
            ),
            key(NearbyService::InterstellarFactors, PadSize::Medium),
        ];
        for (i, a) in all.iter().enumerate() {
            assert_eq!(a.len(), 1);
            for b in &all[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn material_trader_cached_one_hour() {
        let k = cache_keys(&NetQuery::NearestService {
            system_address: 7,
            system_name: "Sol".into(),
            service: NearbyService::MaterialTrader(Some(TraderKind::Raw)),
            min_pad: PadSize::Small,
        });
        let ttl = ttl_for(k[0].0);
        assert_eq!(ttl.fresh, Duration::from_secs(3600));
        assert_eq!(ttl.max_stale, Duration::from_secs(3600));
    }

    #[test]
    fn routes_and_autocomplete_have_no_cache_keys() {
        let route = NetQuery::NeutronRoute(NeutronRouteParams {
            from: "Sol".into(),
            to: "Colonia".into(),
            range_ly: 50.0,
            efficiency: 60,
            supercharge_multiplier: 4.0,
        });
        assert!(cache_keys(&route).is_empty());
        assert!(
            cache_keys(&NetQuery::Autocomplete {
                prefix: "So".into()
            })
            .is_empty()
        );
        assert_eq!(
            cache_keys(&NetQuery::System {
                id64: 5,
                name: "X".into()
            }),
            vec![
                (CacheKind::SystemCard, "spansh:system:5:card".to_string()),
                (
                    CacheKind::SystemPolitics,
                    "spansh:system:5:politics".to_string()
                ),
            ]
        );
    }

    #[test]
    fn trade_exports_and_coords_policies_and_keys() {
        let q = |cap| {
            NetQuery::TradeRoute(tmc_model::net::TradeRouteParams {
                system: "Sol".into(),
                station: "Abraham Lincoln".into(),
                starting_capital: cap,
                max_cargo: 256,
                max_hops: 3,
                max_hop_distance_ly: 30.0,
                min_pad: PadSize::Large,
                max_price_age_days: 1,
                allow_planetary: false,
            })
        };
        let quarter = Duration::from_secs(15 * 60);
        assert_eq!(ttl_for(CacheKind::SpanshTrade).fresh, quarter);
        assert_eq!(ttl_for(CacheKind::ArdentExports).max_stale, quarter);
        assert_eq!(
            ttl_for(CacheKind::SystemCoords).fresh,
            Duration::from_secs(30 * 24 * H)
        );
        let (a, b) = (cache_keys(&q(1)), cache_keys(&q(2)));
        assert_eq!(a, cache_keys(&q(1)));
        assert_ne!(a, b);
        assert!(a[0].1.starts_with("spansh:trade:"));
        assert_eq!(
            cache_keys(&NetQuery::CommodityExportsNearby {
                system_address: 7,
                commodity: "Gold".into(),
                max_distance_ly: 100,
                min_pad: PadSize::Large,
            }),
            vec![(
                CacheKind::ArdentExports,
                "ardent:exports:7:gold:100:l".to_string()
            )]
        );
        assert_eq!(
            cache_keys(&NetQuery::SystemCoordsByName {
                name: " Hip 1 ".into()
            }),
            vec![(CacheKind::SystemCoords, "coords:hip 1".to_string())]
        );
    }
}
