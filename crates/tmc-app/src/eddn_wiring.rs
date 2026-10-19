//! Cableado de EDDN: observa el núcleo y los ajustes, alimenta el filtro y
//! escribe las coordenadas propias en vivo.

use crate::eddn_service::{eddn_wanted, spawn_eddn_service};
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use tmc_core::CoreMessage;
use tmc_eddn::{EddnFilter, EddnMode, ZmqSource};
use tmc_model::settings::ModuleSettings;
use tmc_model::source::SourceEvent;
use tmc_model::state::Galaxy;
use tmc_store::Store;
use tokio::sync::{broadcast, broadcast::error::RecvError, mpsc, watch};

/// Módulos cuya actividad mantiene EDDN conectado en modo `OnDemand`.
const EDDN_MODULES: [&str; 2] = ["trade", "mining"];
/// Modo elegido por el spike de F3.
const EDDN_MODE: EddnMode = EddnMode::OnDemand;
const GRACE: chrono::Duration = chrono::Duration::seconds(60);

/// Sistema propio: nombre, `SystemAddress` y `StarPos`.
type OwnSystem = (String, u64, [f64; 3]);

/// Lo que se sabe del núcleo para decidir el filtro de EDDN.
struct EddnView {
    settings: ModuleSettings,
    galaxy: Galaxy,
    /// EDDN solo con el juego en marcha (nunca durante un replay de diarios).
    game_running: bool,
    system: String,
    system_address: u64,
    star_pos: [f64; 3],
    active: bool,
    inactive_since: Option<DateTime<Utc>>,
}

impl EddnView {
    fn new(settings: ModuleSettings) -> Self {
        Self {
            settings,
            galaxy: Galaxy::Unknown,
            game_running: false,
            system: String::new(),
            system_address: 0,
            star_pos: [0.0; 3],
            active: false,
            inactive_since: None,
        }
    }

    fn center(&self) -> Option<[f64; 3]> {
        (!self.system.is_empty() && self.system_address != 0).then_some(self.star_pos)
    }

    fn filter(&self, now: DateTime<Utc>) -> EddnFilter {
        let center = self.center();
        EddnFilter {
            // Sin centro conocido no se filtraría por distancia: no se conecta.
            enabled: self.game_running
                && center.is_some()
                && eddn_wanted(
                    &self.settings,
                    self.galaxy,
                    EDDN_MODE,
                    self.active,
                    self.inactive_since,
                    now,
                ),
            center,
            radius_ly: f64::from(self.settings.eddn_radius_ly),
        }
    }

    /// Aplica un `Update` del núcleo. Devuelve el sistema propio si cambió.
    fn apply(
        &mut self,
        topics: &BTreeMap<String, serde_json::Value>,
        now: DateTime<Utc>,
    ) -> Option<OwnSystem> {
        if let Some(g) = topics.get("core.game") {
            if let Some(r) = g.get("running").and_then(|v| v.as_bool()) {
                self.game_running = r;
            }
            self.galaxy = match g.get("galaxy").and_then(|v| v.as_str()) {
                Some("Live") => Galaxy::Live,
                Some("Legacy") => Galaxy::Legacy,
                _ => Galaxy::Unknown,
            };
        }
        if let Some(mods) = topics.get("core.modules").and_then(|v| v.as_array()) {
            let active = mods.iter().any(|m| {
                m.get("id")
                    .and_then(|v| v.as_str())
                    .is_some_and(|id| EDDN_MODULES.contains(&id))
                    && matches!(
                        m.get("relevance").and_then(|v| v.as_str()),
                        Some("active" | "urgent")
                    )
            });
            if active != self.active {
                self.inactive_since = (!active).then_some(now);
                self.active = active;
            }
        }
        let loc = topics.get("core.location")?;
        let system = loc.get("system")?.as_str()?.to_owned();
        let address = loc.get("system_address").and_then(|v| v.as_u64())?;
        let pos = loc.get("star_pos")?.as_array()?;
        let pos = [
            pos.first()?.as_f64()?,
            pos.get(1)?.as_f64()?,
            pos.get(2)?.as_f64()?,
        ];
        let changed = system != self.system || address != self.system_address;
        self.system = system.clone();
        self.system_address = address;
        self.star_pos = pos;
        (changed && self.center().is_some() && pos.iter().all(|v| v.is_finite()))
            .then_some((system, address, pos))
    }
}

/// Alimenta el filtro de EDDN, escribe las coordenadas propias en vivo y lanza el servicio.
pub(crate) fn spawn_eddn_wiring(
    store: Store,
    core_tx: mpsc::Sender<SourceEvent>,
    mut rx: broadcast::Receiver<CoreMessage>,
    mut settings_rx: watch::Receiver<ModuleSettings>,
    url: String,
) {
    let mut view = EddnView::new(settings_rx.borrow_and_update().clone());
    let (filter_tx, filter_rx) = watch::channel(view.filter(Utc::now()));
    match ZmqSource::new() {
        Ok(source) => {
            spawn_eddn_service(
                store.clone(),
                core_tx,
                filter_rx,
                EDDN_MODE,
                Box::new(source),
                url,
            );
        }
        Err(error) => tracing::warn!(%error, "EDDN unavailable: could not create transport"),
    }
    tokio::spawn(async move {
        loop {
            // Despierta solo ante eventos, ajustes o el fin del margen de 60 s.
            let wake = match (view.active, view.inactive_since) {
                (false, Some(t)) if Utc::now() - t < GRACE => {
                    Some((GRACE - (Utc::now() - t)).to_std().unwrap_or_default())
                }
                _ => None,
            };
            let mut own: Option<OwnSystem> = None;
            tokio::select! {
                msg = rx.recv() => match msg {
                    Ok(CoreMessage::Update { topics, .. }) => {
                        own = view.apply(&topics, Utc::now());
                    }
                    Ok(_) | Err(RecvError::Lagged(_)) => {}
                    Err(RecvError::Closed) => return,
                },
                changed = settings_rx.changed() => {
                    if changed.is_err() {
                        return;
                    }
                    view.settings = settings_rx.borrow_and_update().clone();
                }
                _ = async {
                    match wake {
                        Some(d) => tokio::time::sleep(d).await,
                        None => std::future::pending().await,
                    }
                } => {}
            }
            if let Some((name, address, pos)) = own {
                // Aunque EDDN esté desactivado: el diario propio alimenta `system_coords`.
                let st = store.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    if let Err(error) = st.upsert_coords(&[(name, Some(address), pos)], Utc::now())
                    {
                        tracing::warn!(%error, "own system coords write failed");
                    }
                })
                .await;
            }
            let filter = view.filter(Utc::now());
            filter_tx.send_if_modified(|f| {
                let changed = *f != filter;
                if changed {
                    *f = filter;
                }
                changed
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn topics(v: serde_json::Value) -> BTreeMap<String, serde_json::Value> {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn filter_follows_game_galaxy_location_and_activity() {
        let now = Utc::now();
        let mut v = EddnView::new(ModuleSettings::default());
        assert!(!v.filter(now).enabled);
        let own = v.apply(
            &topics(json!({
                "core.game": {"running": true, "galaxy": "Live"},
                "core.location": {"system": "Lave", "system_address": 5, "star_pos": [1.0, 2.0, 3.0]},
                "core.modules": [{"id": "mining", "relevance": "active"}],
            })),
            now,
        );
        assert_eq!(own, Some(("Lave".into(), 5, [1.0, 2.0, 3.0])));
        let f = v.filter(now);
        assert!(f.enabled);
        assert_eq!(f.center, Some([1.0, 2.0, 3.0]));
        assert_eq!(f.radius_ly, 200.0);
        // Inactivo: 60 s de margen y luego se apaga.
        v.apply(
            &topics(json!({"core.modules": [{"id": "mining", "relevance": "normal"}]})),
            now,
        );
        assert!(v.filter(now + chrono::Duration::seconds(59)).enabled);
        assert!(!v.filter(now + chrono::Duration::seconds(61)).enabled);
        // Legacy o juego parado: apagado.
        v.apply(
            &topics(json!({"core.modules": [{"id": "mining", "relevance": "active"}]})),
            now,
        );
        assert!(v.filter(now).enabled);
        v.apply(
            &topics(json!({"core.game": {"running": true, "galaxy": "Legacy"}})),
            now,
        );
        assert!(!v.filter(now).enabled);
        v.apply(
            &topics(json!({"core.game": {"running": false, "galaxy": "Live"}})),
            now,
        );
        assert!(!v.filter(now).enabled);
    }
}
