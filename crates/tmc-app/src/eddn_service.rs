//! Servicio EDDN: escribe en la tabla de precios los lotes del suscriptor,
//! siembra coordenadas desde el diario propio y limpia lo antiguo.

use chrono::{DateTime, Duration as CDuration, Utc};
use std::collections::BTreeSet;
use std::time::Duration;
use tmc_eddn::{
    Clock, CoordEntry, EddnFilter, EddnMode, EddnOutput, FrameSource, SystemClock, spawn_subscriber,
};
use tmc_model::settings::ModuleSettings;
use tmc_model::source::{EDDN_NOTICE_MAX_COMMODITIES, EddnNotice, SourceEvent};
use tmc_model::state::Galaxy;
use tmc_store::Store;
use tmc_store::prices::{COORDS_RETENTION_DAYS, PRICE_RETENTION_DAYS};
use tokio::sync::{mpsc, watch};

/// Margen de EDDN activo tras dejar de ser necesario (modo `OnDemand`).
const GRACE_SECS: i64 = 60;
/// Periodo de la limpieza (el temporizador solo despierta una vez por periodo).
const PURGE_EVERY: Duration = Duration::from_secs(3600);
/// Espera máxima a conocer el centro antes de sembrar el índice.
const CENTER_WAIT: Duration = Duration::from_secs(30);
const COORD_SEED_MARGIN_LY: f64 = 100.0;

/// Si EDDN debe estar conectado ahora (sin servicios en línea o en Legacy, nunca).
pub fn eddn_wanted(
    s: &ModuleSettings,
    galaxy: Galaxy,
    mode: EddnMode,
    trade_or_mining_active: bool,
    inactive_since: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> bool {
    if !s.online_services || galaxy != Galaxy::Live {
        return false;
    }
    match mode {
        EddnMode::Always => true,
        EddnMode::OnDemand => {
            trade_or_mining_active
                || inactive_since.is_some_and(|t| now - t < CDuration::seconds(GRACE_SECS))
        }
    }
}

pub fn spawn_eddn_service(
    store: Store,
    core_tx: mpsc::Sender<SourceEvent>,
    filter_rx: watch::Receiver<EddnFilter>,
    mode: EddnMode,
    source: Box<dyn FrameSource>,
    url: String,
) -> tokio::task::JoinHandle<()> {
    spawn_with_clock(
        store,
        core_tx,
        filter_rx,
        mode,
        source,
        url,
        Box::new(SystemClock),
    )
}

pub(crate) fn spawn_with_clock(
    store: Store,
    core_tx: mpsc::Sender<SourceEvent>,
    filter_rx: watch::Receiver<EddnFilter>,
    mode: EddnMode,
    source: Box<dyn FrameSource>,
    url: String,
    clock: Box<dyn Clock>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(run(store, core_tx, filter_rx, mode, source, url, clock))
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    tokio::task::spawn_blocking(f).await.ok()
}

async fn purge(store: &Store) {
    let st = store.clone();
    let now = Utc::now();
    let r = blocking(move || {
        st.purge_prices_older_than(
            now - CDuration::days(PRICE_RETENTION_DAYS),
            now - CDuration::days(COORDS_RETENTION_DAYS),
        )
    })
    .await;
    match r {
        Some(Ok(c)) => {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::State,
                "eddn.purge.completed",
                serde_json::json!({"prices":c.prices,"stations":c.stations,"coords":c.coords})
            );
            tracing::debug!(?c, "EDDN purge done");
        }
        Some(Err(error)) => {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::State,
                "eddn.purge.failed",
                serde_json::json!({"failure_class":error.diagnostic_class()})
            );
            tracing::warn!(%error, "EDDN purge failed");
        }
        None => {}
    }
}

/// Copia las coordenadas propias al arrancar y al terminar la importación histórica.
/// Se llama desde un worker, nunca desde el bucle asíncrono. Siembra las coordenadas propias con `seen_at = ahora`, por lo que sobreviven a la purga de 30 días
/// mientras se re-siembren al arrancar y tras cada importación. Se omiten los diarios de Legacy
/// (`journal_file.galaxy`), cuyo espacio de coordenadas no es el de Live.
pub(crate) fn seed_own_jumps_blocking(store: &Store) {
    let r = (|| -> Result<usize, tmc_store::StoreError> {
        let rows = store.query_json(
            "SELECT json_extract(e.json,'$.StarSystem'), json_extract(e.json,'$.SystemAddress'),              json_extract(e.json,'$.StarPos[0]'), json_extract(e.json,'$.StarPos[1]'),              json_extract(e.json,'$.StarPos[2]')              FROM event e LEFT JOIN journal_file f ON f.id=e.file_id              WHERE e.name IN ('FSDJump','Location','CarrierJump')              AND json_extract(e.json,'$.StarPos[2]') IS NOT NULL              AND COALESCE(f.galaxy,'')<>'Legacy'              GROUP BY json_extract(e.json,'$.StarSystem')",
            &[],
        )?;
        let coords: Vec<CoordEntry> = rows
            .iter()
            .filter_map(|row| {
                let name = row[0].as_str().filter(|n| !n.is_empty())?;
                let pos = [row[2].as_f64()?, row[3].as_f64()?, row[4].as_f64()?];
                pos.iter().all(|v| v.is_finite()).then_some(())?;
                Some((name.to_owned(), row[1].as_u64(), pos))
            })
            .collect();
        store.upsert_coords(&coords, Utc::now())
    })();
    if let Err(error) = r {
        tracing::warn!(%error, "seeding system coords from own journal failed");
    }
}

async fn seed_own_jumps(store: &Store) {
    let st = store.clone();
    blocking(move || seed_own_jumps_blocking(&st)).await;
}

async fn run(
    store: Store,
    core_tx: mpsc::Sender<SourceEvent>,
    mut filter_rx: watch::Receiver<EddnFilter>,
    mode: EddnMode,
    source: Box<dyn FrameSource>,
    url: String,
    clock: Box<dyn Clock>,
) {
    purge(&store).await;
    seed_own_jumps(&store).await;

    // Sin centro no hay a qué sembrar: se espera un tiempo acotado a conocerlo.
    let area =
        match tokio::time::timeout(CENTER_WAIT, filter_rx.wait_for(|f| f.center.is_some())).await {
            Ok(Ok(f)) => f.center.map(|c| (c, f.radius_ly)),
            Ok(Err(_)) => return,
            Err(_) => None,
        };
    let seed: Vec<CoordEntry> = match area {
        Some((c, r)) if r.is_finite() && r >= 0.0 => {
            let st = store.clone();
            let found = blocking(move || {
                st.coords_within(c, r + COORD_SEED_MARGIN_LY, tmc_eddn::COORD_INDEX_CAP)
            })
            .await;
            match found {
                Some(Ok(v)) => v.into_iter().map(|(k, p)| (k, None, p)).collect(),
                Some(Err(error)) => {
                    tracing::warn!(%error, "seeding EDDN coord index failed");
                    Vec::new()
                }
                None => Vec::new(),
            }
        }
        _ => Vec::new(),
    };
    tracing::debug!(?mode, seeds = seed.len(), "EDDN service starting");
    tmc_model::diagnostic!(
        tmc_model::diagnostics::DiagnosticContext::State,
        "eddn.service.started",
        serde_json::json!({"on_demand":matches!(mode, EddnMode::OnDemand),"seeds":seed.len(),"center_known":area.is_some()})
    );

    let (tx, mut rx) = mpsc::channel::<EddnOutput>(32);
    // El hilo termina solo al cerrarse `rx` (se suelta al acabar esta tarea).
    drop(spawn_subscriber(source, clock, url, filter_rx, tx, seed));

    let mut tick = tokio::time::interval_at(tokio::time::Instant::now() + PURGE_EVERY, PURGE_EVERY);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut connected = false;
    loop {
        tokio::select! {
            out = rx.recv() => {
                let Some(out) = out else { return };
                #[cfg(debug_assertions)]
                if let EddnOutput::Status(s) = &out {
                    let corrupt = s
                        .stats
                        .dropped
                        .get(&tmc_eddn::DropReason::Corrupt)
                        .copied()
                        .unwrap_or(0);
                    E2E_CORRUPT_DROPS.fetch_max(corrupt, std::sync::atomic::Ordering::Relaxed);
                }
                let notice = match out {
                    EddnOutput::Batch(markets, stations, coords) => {
                        let ids: Vec<u64> = markets.iter().map(|m| m.market_id).collect();
                        let mut keys = BTreeSet::new();
                        for m in &markets {
                            keys.extend(m.prices.iter().map(|p| p.commodity.clone()));
                        }
                        let st = store.clone();
                        let now = Utc::now();
                        let written = blocking(move || -> Result<(), tmc_store::StoreError> {
                            st.replace_markets(&markets)?;
                            st.upsert_stations(&stations, now)?;
                            st.upsert_coords(&coords, now)?;
                            Ok(())
                        })
                        .await;
                        tmc_model::diagnostic!(
                            tmc_model::diagnostics::DiagnosticContext::State,
                            "eddn.batch.received",
                            serde_json::json!({"markets":ids.len(),"commodities":keys.len(),"written":matches!(written, Some(Ok(())))})
                        );
                        match written {
                            Some(Ok(())) => Some(EddnNotice {
                                markets: ids,
                                commodities: keys
                                    .into_iter()
                                    .take(EDDN_NOTICE_MAX_COMMODITIES)
                                    .collect(),
                                connected,
                                at: now,
                            }),
                            Some(Err(error)) => {
                                tmc_model::diagnostic!(
                                    tmc_model::diagnostics::DiagnosticContext::State,
                                    "eddn.batch.write_failed",
                                    serde_json::json!({"failure_class":error.diagnostic_class()})
                                );
                                tracing::warn!(%error, "EDDN batch write failed");
                                None
                            }
                            None => None,
                        }
                    }
                    EddnOutput::Status(s) if s.connected != connected => {
                        connected = s.connected;
                        tmc_model::diagnostic!(
                            tmc_model::diagnostics::DiagnosticContext::State,
                            "eddn.connection.changed",
                            serde_json::json!({"connected":connected})
                        );
                        Some(EddnNotice {
                            markets: Vec::new(),
                            commodities: Vec::new(),
                            connected,
                            at: Utc::now(),
                        })
                    }
                    EddnOutput::Status(_) => None,
                };
                if let Some(n) = notice
                    && core_tx.send(SourceEvent::Eddn(n)).await.is_err()
                {
                    return;
                }
            }
            _ = tick.tick() => purge(&store).await,
        }
    }
}

/// Solo depuración (pruebas e2e): mayor cifra de mensajes corruptos descartados vista en el estado.
#[cfg(debug_assertions)]
pub(crate) static E2E_CORRUPT_DROPS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{Compression, write::ZlibEncoder};
    use std::collections::VecDeque;
    use std::io::Write;
    use tmc_eddn::{COMMODITY_SCHEMA, EddnError, FakeClock};
    use tmc_model::market::{CommodityPrice, MarketSnapshot, PriceOrigin};

    fn settings(online: bool) -> ModuleSettings {
        ModuleSettings {
            online_services: online,
            ..ModuleSettings::default()
        }
    }

    #[test]
    fn eddn_disabled_in_legacy_or_offline() {
        let now = Utc::now();
        assert!(!eddn_wanted(
            &settings(true),
            Galaxy::Legacy,
            EddnMode::Always,
            true,
            None,
            now
        ));
        assert!(!eddn_wanted(
            &settings(false),
            Galaxy::Live,
            EddnMode::Always,
            true,
            None,
            now
        ));
        assert!(eddn_wanted(
            &settings(true),
            Galaxy::Live,
            EddnMode::Always,
            false,
            None,
            now
        ));
    }

    #[test]
    fn on_demand_keeps_60s_after_inactive() {
        let now = Utc::now();
        let s = settings(true);
        let wanted = |secs: i64, active: bool| {
            eddn_wanted(
                &s,
                Galaxy::Live,
                EddnMode::OnDemand,
                active,
                Some(now - CDuration::seconds(secs)),
                now,
            )
        };
        assert!(wanted(59, false));
        assert!(!wanted(61, false));
        assert!(wanted(600, true));
        assert!(!eddn_wanted(
            &s,
            Galaxy::Live,
            EddnMode::OnDemand,
            false,
            None,
            now
        ));
    }

    struct Fake {
        frames: VecDeque<Vec<u8>>,
        clock: FakeClock,
    }
    impl FrameSource for Fake {
        fn connect(&mut self, _url: &str) -> Result<(), EddnError> {
            Ok(())
        }
        fn recv(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>, EddnError> {
            if let Some(f) = self.frames.pop_front() {
                return Ok(Some(f));
            }
            self.clock.advance(timeout);
            std::thread::sleep(Duration::from_millis(2));
            Ok(None)
        }
        fn disconnect(&mut self) {}
    }

    fn frame(id: u64) -> Vec<u8> {
        let ts = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let json = format!(
            r#"{{"$schemaRef":"{COMMODITY_SCHEMA}","header":{{}},"message":{{"timestamp":"{ts}","systemName":"Lave","stationName":"Puerto {id}","marketId":{id},"commodities":[{{"name":"explosives","meanPrice":483,"buyPrice":203,"stock":58295,"stockBracket":3,"sellPrice":183,"demand":1,"demandBracket":0}}]}}}}"#
        );
        let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
        e.write_all(json.as_bytes()).unwrap();
        e.finish().unwrap()
    }

    fn filter(enabled: bool) -> EddnFilter {
        EddnFilter {
            enabled,
            center: Some([0.0; 3]),
            radius_ly: 50.0,
        }
    }

    fn start(
        store: &Store,
        frames: Vec<Vec<u8>>,
        f: EddnFilter,
    ) -> (
        mpsc::Receiver<SourceEvent>,
        watch::Sender<EddnFilter>,
        tokio::task::JoinHandle<()>,
    ) {
        let clock = FakeClock::new();
        let (core_tx, core_rx) = mpsc::channel(64);
        let (ftx, frx) = watch::channel(f);
        let h = spawn_with_clock(
            store.clone(),
            core_tx,
            frx,
            EddnMode::Always,
            Box::new(Fake {
                frames: frames.into(),
                clock: clock.clone(),
            }),
            "tcp://fake".into(),
            Box::new(clock),
        );
        (core_rx, ftx, h)
    }

    #[tokio::test]
    async fn batch_is_written_and_notified() {
        let store = Store::open_in_memory().unwrap();
        store
            .upsert_coords(&[("Lave".into(), None, [1.0, 0.0, 0.0])], Utc::now())
            .unwrap();
        let (mut rx, _ftx, h) = start(&store, vec![frame(1), frame(2), frame(3)], filter(true));
        let notice = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                if let Some(SourceEvent::Eddn(n)) = rx.recv().await
                    && !n.markets.is_empty()
                {
                    return n;
                }
            }
        })
        .await
        .expect("sin aviso EDDN");
        assert_eq!(notice.markets.len(), 3);
        assert_eq!(notice.commodities, vec!["explosives".to_owned()]);
        assert!(!store.market_prices(1).unwrap().is_empty());
        h.abort();
    }

    #[tokio::test]
    async fn purge_runs_at_start() {
        let store = Store::open_in_memory().unwrap();
        let old = Utc::now() - CDuration::days(8);
        store
            .replace_markets(&[MarketSnapshot {
                market_id: 9,
                station_name: "Vieja".into(),
                system_name: "Lave".into(),
                system_address: None,
                star_pos: None,
                max_pad: None,
                station_type: None,
                updated_at: old,
                origin: PriceOrigin::Eddn,
                prices: vec![CommodityPrice {
                    commodity: "gold".into(),
                    name_localised: None,
                    buy_price: 1,
                    sell_price: 1,
                    stock: 1,
                    demand: 1,
                }],
            }])
            .unwrap();
        assert_eq!(store.market_prices(9).unwrap().len(), 1);
        let (_rx, _ftx, h) = start(&store, vec![], filter(false));
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !store.market_prices(9).unwrap().is_empty() {
            assert!(std::time::Instant::now() < deadline, "no purgó");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        h.abort();
    }

    #[tokio::test]
    async fn own_coords_are_seeded_after_history_import_finishes() {
        let store = Store::open_in_memory().unwrap();
        // Startup can finish its first seed before the background import has any rows.
        seed_own_jumps(&store).await;
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Journal.2026-10-18T000000.01.log"),
            concat!(
                "{\"timestamp\":\"2026-10-18T10:00:00Z\",\"event\":\"FSDJump\",\"StarSystem\":\"Lave\",\"SystemAddress\":1,\"StarPos\":[75.75,48.75,70.75]}\n",
                "{\"timestamp\":\"2026-10-18T10:01:00Z\",\"event\":\"Location\",\"StarSystem\":\"Sol\",\"SystemAddress\":2,\"StarPos\":[0,0,0]}\n",
                "{\"timestamp\":\"2026-10-18T10:02:00Z\",\"event\":\"CarrierJump\",\"StarSystem\":\"Achenar\",\"SystemAddress\":3,\"StarPos\":[67.5,-119.5,24.8125]}\n",
            ),
        )
        .unwrap();
        let report = tmc_store::import::import_history(
            &store,
            dir.path(),
            &tmc_store::import::ImportOptions {
                throttle: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            },
            &mut |_, _| {},
        )
        .unwrap();
        assert_eq!(report.events, 3);
        let names = ["lave".into(), "sol".into(), "achenar".into()];
        assert!(store.coords_by_names(&names).unwrap().is_empty());

        seed_own_jumps_blocking(&store);

        let coords = store.coords_by_names(&names).unwrap();
        assert_eq!(coords.len(), 3);
        assert_eq!(coords["lave"], [75.75, 48.75, 70.75]);
        assert_eq!(coords["sol"], [0.0, 0.0, 0.0]);
        assert_eq!(coords["achenar"], [67.5, -119.5, 24.8125]);
    }

    #[tokio::test]
    async fn own_jumps_seed_system_coords() {
        let store = Store::open_in_memory().unwrap();
        store
            .exec_json(
                "INSERT INTO journal_file(id,name,size,offset) VALUES(1,'j.log',0,0)",
                &[],
            )
            .unwrap();
        store
            .exec_json(
                "INSERT INTO event(file_id,line,ts,name,json) VALUES(1,1,'2026-10-18T10:00:00Z','FSDJump',?1)",
                &[serde_json::json!(
                    r#"{"event":"FSDJump","StarSystem":"Lave","SystemAddress":3932277478106,"StarPos":[75.75,48.75,70.75]}"#
                )],
            )
            .unwrap();
        let (_rx, _ftx, h) = start(&store, vec![], filter(false));
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            let got = store.coords_by_names(&["lave".to_owned()]).unwrap();
            if !got.is_empty() {
                assert_eq!(got["lave"], [75.75, 48.75, 70.75]);
                break;
            }
            assert!(std::time::Instant::now() < deadline, "sin coordenadas");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        h.abort();
    }

    #[test]
    fn seed_skips_legacy_files_and_survives_purge() {
        let store = Store::open_in_memory().unwrap();
        for (id, gal) in [(1, "Live"), (2, "Legacy")] {
            store
                .exec_json(
                    "INSERT INTO journal_file(id,name,size,offset,galaxy) VALUES(?1,?2,0,0,?3)",
                    &[
                        serde_json::json!(id),
                        serde_json::json!(format!("j{id}.log")),
                        serde_json::json!(gal),
                    ],
                )
                .unwrap();
        }
        for (id, name) in [(1, "Lave"), (2, "Legacyville")] {
            store
                .exec_json(
                    "INSERT INTO event(file_id,line,ts,name,json) VALUES(?1,1,'2026-10-18T10:00:00Z','FSDJump',?2)",
                    &[
                        serde_json::json!(id),
                        serde_json::json!(format!(
                            r#"{{"event":"FSDJump","StarSystem":"{name}","SystemAddress":1,"StarPos":[1.0,2.0,3.0]}}"#
                        )),
                    ],
                )
                .unwrap();
        }
        seed_own_jumps_blocking(&store);
        let names = ["lave".to_owned(), "legacyville".to_owned()];
        let got = store.coords_by_names(&names).unwrap();
        assert!(got.contains_key("lave") && !got.contains_key("legacyville"));
        // La purga las borra solo cuando son antiguas; la re-siembra las restituye.
        let far = Utc::now() + chrono::Duration::days(365);
        store.purge_prices_older_than(far, far).unwrap();
        assert!(store.coords_by_names(&names).unwrap().is_empty());
        seed_own_jumps_blocking(&store);
        assert!(store.coords_by_names(&names).unwrap().contains_key("lave"));
    }
}
