//! Política de caducidad de la caché y claves por consulta.

use crate::ardent;
use std::time::Duration;
use tmc_model::net::{NearbyService, NetQuery, PadSize};

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
    }
}

fn pad_tag(p: PadSize) -> &'static str {
    match p {
        PadSize::Small => "s",
        PadSize::Medium => "m",
        PadSize::Large => "l",
    }
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
        NetQuery::NeutronRoute(_)
        | NetQuery::GalaxyRoute(_)
        | NetQuery::Autocomplete { .. }
        | NetQuery::TradeRoute(_)
        | NetQuery::CommodityExportsNearby { .. }
        | NetQuery::SystemCoordsByName { .. } => Vec::new(),
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
        } => vec![(
            CacheKind::ArdentNearest,
            format!(
                "ardent:nearest:{system_address}:{}:{}",
                match service {
                    NearbyService::InterstellarFactors => "interstellar-factors",
                },
                pad_tag(*min_pad)
            ),
        )],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tmc_model::net::{NearbyService, NetQuery, NeutronRouteParams, PadSize};

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
}
