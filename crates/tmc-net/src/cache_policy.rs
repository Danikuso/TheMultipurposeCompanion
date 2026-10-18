//! Política de caducidad de la caché y claves por consulta.

use std::time::Duration;
use tmc_model::net::NetQuery;

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
        // Ardent: sin caché hasta la Task 7.
        | NetQuery::CommodityImportsNearby { .. }
        | NetQuery::NearestService { .. } => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tmc_model::net::{NetQuery, NeutronRouteParams};

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
