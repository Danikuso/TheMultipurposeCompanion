//! Agregador de estado honesto para la UI (`SyncReport`, DR-8, E0-12).

use crate::clock::Clock;
use crate::ids::{cmdr_ref, key_hint};
use crate::router::eddn_enabled_at;
use crate::worker::{ServiceEffect, WorkerEvent};
use chrono::{DateTime, SecondsFormat, Utc};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tmc_model::sync::{
    CmdrRef, CommanderRow, EddnServiceStatus, ErrorClass, KeyAssignment, KeyedServiceStatus,
    PendingByService, RECENT_REJECTIONS_MAX, RecentRejection, RosterReport, SERVER_HINT_MAX_CHARS,
    SyncReason, SyncReport, SyncService, SyncServiceStatus, SyncState,
};
use tmc_store::Store;

pub const EDDN_PAUSED_SCHEMAS_KEY: &str = "sync.eddn.paused_schemas";
const REPORT_MIN_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, PartialEq, Eq)]
enum LastOutcome {
    None,
    Connected,
    ServiceError {
        class: ErrorClass,
        until: Option<DateTime<Utc>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ReportSlot {
    Inara,
    Edsm,
    Eddn,
    Roster,
}

impl ReportSlot {
    fn of(report: &SyncReport) -> Self {
        match report {
            SyncReport::Inara(_) => Self::Inara,
            SyncReport::Edsm(_) => Self::Edsm,
            SyncReport::Eddn(_) => Self::Eddn,
            SyncReport::Roster(_) => Self::Roster,
        }
    }
}

/// Construye instantáneas de estado honestas a partir del almacén y los eventos reales.
pub struct StatusAggregator {
    store: Store,
    _clock: Arc<dyn Clock>,
    salt: String,
    sent_session: [u64; 3],
    rejected_session: [u64; 3],
    recovered_session: [u64; 3],
    service_outcome: [LastOutcome; 3],
    cmdr_error: HashMap<(SyncService, String), ErrorClass>,
    rejections: [VecDeque<RecentRejection>; 3],
    paused_schemas: Vec<String>,
    last_emitted: HashMap<ReportSlot, SyncReport>,
    last_sent_at: HashMap<ReportSlot, tokio::time::Instant>,
}

impl StatusAggregator {
    pub fn new(store: Store, clock: Arc<dyn Clock>, salt: String) -> Self {
        let paused_schemas = read_paused_schemas(&store);
        Self {
            store,
            _clock: clock,
            salt,
            sent_session: [0; 3],
            rejected_session: [0; 3],
            recovered_session: [0; 3],
            service_outcome: [LastOutcome::None, LastOutcome::None, LastOutcome::None],
            cmdr_error: HashMap::new(),
            rejections: [VecDeque::new(), VecDeque::new(), VecDeque::new()],
            paused_schemas,
            last_emitted: HashMap::new(),
            last_sent_at: HashMap::new(),
        }
    }

    pub fn on_worker_event(&mut self, ev: &WorkerEvent) {
        match ev {
            WorkerEvent::Outcome {
                service,
                cmdr_fid,
                effect,
                sent,
                rejected,
            } => {
                let idx = service_idx(*service);
                self.sent_session[idx] += *sent;
                self.rejected_session[idx] += *rejected;
                match effect {
                    ServiceEffect::None => {}
                    ServiceEffect::Connected => {
                        self.service_outcome[idx] = LastOutcome::Connected;
                        self.cmdr_error.remove(&(*service, cmdr_fid.clone()));
                    }
                    ServiceEffect::CmdrError(class) => {
                        self.cmdr_error.insert((*service, cmdr_fid.clone()), *class);
                    }
                    ServiceEffect::ServiceError { class, until } => {
                        self.service_outcome[idx] = LastOutcome::ServiceError {
                            class: *class,
                            until: *until,
                        };
                    }
                    ServiceEffect::PauseSchema(schema) => {
                        if !self.paused_schemas.iter().any(|s| s == schema) {
                            self.paused_schemas.push(schema.clone());
                            self.paused_schemas.sort();
                        }
                    }
                }
            }
            WorkerEvent::Rejection {
                service, rejection, ..
            } => {
                let idx = service_idx(*service);
                let mut item = rejection.clone();
                if let Some(hint) = item.hint {
                    item.hint = Some(hint.chars().take(SERVER_HINT_MAX_CHARS).collect());
                }
                let queue = &mut self.rejections[idx];
                queue.push_front(item);
                while queue.len() > RECENT_REJECTIONS_MAX {
                    queue.pop_back();
                }
            }
        }
    }

    pub fn on_recovered(&mut self, service: SyncService, n: u64) {
        self.recovered_session[service_idx(service)] += n;
    }

    /// Genera los 4 reportes actuales (`Inara`, `Edsm`, `Eddn`, `Roster`).
    pub fn build(
        &self,
        current_fid: Option<&str>,
        blocked: &dyn Fn(SyncService, Option<&str>) -> Option<SyncReason>,
    ) -> Vec<SyncReport> {
        let inara = self.build_keyed(SyncService::Inara, current_fid, blocked);
        let edsm = self.build_keyed(SyncService::Edsm, current_fid, blocked);
        let eddn = self.build_eddn(current_fid, blocked);
        let roster = self.build_roster(current_fid);
        vec![
            SyncReport::Inara(inara),
            SyncReport::Edsm(edsm),
            SyncReport::Eddn(eddn),
            SyncReport::Roster(roster),
        ]
    }

    /// Emite solo los reportes que han cambiado y respetan el máximo de 1 actualización/s por servicio.
    pub fn changed_reports(
        &mut self,
        reports: Vec<SyncReport>,
        now: tokio::time::Instant,
    ) -> Vec<SyncReport> {
        let mut out = Vec::new();
        for report in reports {
            let slot = ReportSlot::of(&report);
            if self.last_emitted.get(&slot) == Some(&report) {
                continue;
            }
            if let Some(&prev_at) = self.last_sent_at.get(&slot)
                && now.saturating_duration_since(prev_at) < REPORT_MIN_INTERVAL
            {
                continue;
            }
            self.last_emitted.insert(slot, report.clone());
            self.last_sent_at.insert(slot, now);
            out.push(report);
        }
        out
    }

    fn build_service_status(
        &self,
        service: SyncService,
        current_fid: Option<&str>,
        blocked: &dyn Fn(SyncService, Option<&str>) -> Option<SyncReason>,
    ) -> SyncServiceStatus {
        let idx = service_idx(service);
        let counts = current_fid
            .and_then(|fid| self.store.sync_counts(service, fid).ok())
            .unwrap_or_default();
        let (state, reason, next_attempt_at) = self.derive_state(service, current_fid, blocked);
        SyncServiceStatus {
            state,
            reason,
            pending: counts.pending,
            recovered_session: self.recovered_session[idx],
            sent_total: counts.sent_total,
            sent_session: self.sent_session[idx],
            rejected_total: counts.rejected_total,
            rejected_session: self.rejected_session[idx],
            expired_total: counts.expired_total,
            omitted_total: counts.omitted_total,
            last_confirmed_at: counts.last_confirmed_at,
            next_attempt_at,
            recent_rejections: self.rejections[idx].iter().cloned().collect(),
        }
    }

    fn derive_state(
        &self,
        service: SyncService,
        current_fid: Option<&str>,
        blocked: &dyn Fn(SyncService, Option<&str>) -> Option<SyncReason>,
    ) -> (SyncState, Option<SyncReason>, Option<String>) {
        if let Some(reason) = blocked(service, current_fid) {
            let state = match reason {
                SyncReason::PlatformUnsupported => SyncState::Unavailable,
                SyncReason::NoKey | SyncReason::SwitchOff => SyncState::Disabled,
                _ => SyncState::Paused,
            };
            return (state, Some(reason), None);
        }

        let idx = service_idx(service);
        if let Some(fid) = current_fid
            && let Some(class) = self.cmdr_error.get(&(service, fid.to_string())).copied()
        {
            let next_at = self
                .store
                .next_attempt_at_for(service, Some(fid))
                .ok()
                .flatten()
                .map(|t| t.to_rfc3339_opts(SecondsFormat::Secs, true));
            return (SyncState::Error, class_to_reason(class), next_at);
        }

        match self.service_outcome[idx] {
            LastOutcome::ServiceError { class, until } => {
                let next_at = until
                    .or_else(|| self.store.next_attempt_at(service).ok().flatten())
                    .map(|t| t.to_rfc3339_opts(SecondsFormat::Secs, true));
                (SyncState::Error, class_to_reason(class), next_at)
            }
            LastOutcome::Connected => (SyncState::Connected, None, None),
            LastOutcome::None => (SyncState::Waiting, None, None),
        }
    }

    fn build_keyed(
        &self,
        service: SyncService,
        current_fid: Option<&str>,
        blocked: &dyn Fn(SyncService, Option<&str>) -> Option<SyncReason>,
    ) -> KeyedServiceStatus {
        let cred =
            current_fid.and_then(|fid| self.store.sync_credential(service, fid).ok().flatten());
        KeyedServiceStatus {
            status: self.build_service_status(service, current_fid, blocked),
            has_key: cred.is_some(),
            key_hint: cred.map(|c| key_hint(&c.api_key)),
        }
    }

    fn build_eddn(
        &self,
        current_fid: Option<&str>,
        blocked: &dyn Fn(SyncService, Option<&str>) -> Option<SyncReason>,
    ) -> EddnServiceStatus {
        let mut paused = read_paused_schemas(&self.store);
        for s in &self.paused_schemas {
            if !paused.iter().any(|x| x == s) {
                paused.push(s.clone());
            }
        }
        paused.sort();
        EddnServiceStatus {
            status: self.build_service_status(SyncService::Eddn, current_fid, blocked),
            enabled: eddn_enabled_at(&self.store).is_some(),
            paused_schemas: paused,
        }
    }

    fn build_roster(&self, current_fid: Option<&str>) -> RosterReport {
        let known = self.store.known_cmdrs().unwrap_or_default();
        let current_cmdr = current_fid.map(|fid| {
            let name = known
                .iter()
                .find(|(f, _)| f == fid)
                .map(|(_, n)| n.clone())
                .unwrap_or_default();
            CmdrRef {
                cmdr_ref: cmdr_ref(&self.salt, fid),
                name,
            }
        });

        let mut commanders = Vec::with_capacity(known.len());
        for (fid, name) in known {
            let inara_cred = self
                .store
                .sync_credential(SyncService::Inara, &fid)
                .ok()
                .flatten();
            let edsm_cred = self
                .store
                .sync_credential(SyncService::Edsm, &fid)
                .ok()
                .flatten();
            let key_assignment = match (&inara_cred, &edsm_cred) {
                (Some(c), _) | (_, Some(c))
                    if c.assignment == KeyAssignment::AutoPendingConfirmation =>
                {
                    Some(KeyAssignment::AutoPendingConfirmation)
                }
                (Some(_), _) | (_, Some(_)) => Some(KeyAssignment::Confirmed),
                (None, None) => None,
            };
            let pending_of = |svc| {
                self.store
                    .sync_counts(svc, &fid)
                    .map(|c| c.pending)
                    .unwrap_or(0)
            };
            commanders.push(CommanderRow {
                cmdr_ref: cmdr_ref(&self.salt, &fid),
                name,
                inara_key: inara_cred.is_some(),
                edsm_key: edsm_cred.is_some(),
                key_assignment,
                inara_pending: pending_of(SyncService::Inara),
                edsm_pending: pending_of(SyncService::Edsm),
                eddn_pending: pending_of(SyncService::Eddn),
            });
        }

        let other_pending = |svc| -> u64 {
            self.store
                .pending_by_cmdr(svc)
                .unwrap_or_default()
                .into_iter()
                .filter(|(fid, _, _)| Some(fid.as_str()) != current_fid)
                .map(|(_, count, _)| count)
                .sum()
        };

        let unassigned_keys = self
            .store
            .unassigned_keys()
            .unwrap_or_default()
            .into_iter()
            .map(|k| k.service)
            .collect();

        RosterReport {
            current_cmdr,
            commanders,
            other_cmdrs_pending: PendingByService {
                inara: other_pending(SyncService::Inara),
                edsm: other_pending(SyncService::Edsm),
                eddn: other_pending(SyncService::Eddn),
            },
            unassigned_keys,
        }
    }
}

fn service_idx(service: SyncService) -> usize {
    match service {
        SyncService::Inara => 0,
        SyncService::Edsm => 1,
        SyncService::Eddn => 2,
    }
}

fn class_to_reason(class: ErrorClass) -> Option<SyncReason> {
    Some(match class {
        ErrorClass::Network => SyncReason::Network,
        ErrorClass::Timeout => SyncReason::Timeout,
        ErrorClass::RateLimited => SyncReason::RateLimited,
        ErrorClass::Blocked => SyncReason::Blocked,
        ErrorClass::Auth => SyncReason::Auth,
        ErrorClass::SoftwareBlocked => SyncReason::SoftwareBlocked,
        ErrorClass::SchemaOutdated => SyncReason::SchemaOutdated,
        ErrorClass::ServerError => SyncReason::ServerError,
        ErrorClass::DiscardListMissing => SyncReason::DiscardListMissing,
        ErrorClass::InvalidEvent | ErrorClass::TooLarge => return None,
    })
}

fn read_paused_schemas(store: &Store) -> Vec<String> {
    store
        .kv_get(EDDN_PAUSED_SCHEMAS_KEY)
        .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::TestClock;
    use chrono::TimeZone;
    use tmc_model::sync::{OutboxState, RowOrigin};
    use tmc_store::{NewOutboxRow, SyncCredential};

    const F1: &str = "F0000001";
    const F2: &str = "F0000002";

    fn setup() -> (Store, Arc<TestClock>, StatusAggregator) {
        let store = Store::open_in_memory().unwrap();
        let now = Utc.with_ymd_and_hms(2026, 3, 10, 12, 0, 0).unwrap();
        let clock = TestClock::new(now);
        store.upsert_known_cmdr(F1, "CMDR ALPHA", now).unwrap();
        store
            .put_sync_credential(&SyncCredential {
                service: SyncService::Inara,
                cmdr_fid: F1.into(),
                cmdr_name: "CMDR ALPHA".into(),
                api_key: "INARA-KEY-1234".into(),
                created_at: now.to_rfc3339(),
                verified_at: None,
                assignment: KeyAssignment::Confirmed,
            })
            .unwrap();
        store.set_activation(SyncService::Inara, F1, now).unwrap();
        let agg = StatusAggregator::new(store.clone(), clock.clone(), "salt-test".into());
        (store, clock, agg)
    }

    fn inara_status(reports: &[SyncReport]) -> &KeyedServiceStatus {
        reports
            .iter()
            .find_map(|r| match r {
                SyncReport::Inara(s) => Some(s),
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn active_without_response_is_waiting() {
        let (_store, _clock, agg) = setup();
        let reports = agg.build(Some(F1), &|svc, _| {
            if svc == SyncService::Inara {
                None
            } else {
                Some(SyncReason::NoKey)
            }
        });
        let st = inara_status(&reports);
        assert_eq!(st.status.state, SyncState::Waiting);
        assert_eq!(st.status.sent_total, 0);
        assert_eq!(st.status.last_confirmed_at, None);
        assert!(st.has_key);
        assert_eq!(st.key_hint.as_deref(), Some("••••1234"));
    }

    #[test]
    fn connected_only_after_success_effect() {
        let (store, clock, mut agg) = setup();
        let before = agg.build(Some(F1), &|_, _| None);
        assert_eq!(inara_status(&before).status.state, SyncState::Waiting);

        let now = clock.now_utc();
        store
            .enqueue_batch_with_cursor(
                &[NewOutboxRow {
                    service: SyncService::Inara,
                    cmdr_fid: F1.into(),
                    idem_key: "k1".into(),
                    kind: "FSDJump".into(),
                    coalesce_key: None,
                    event_ts: now,
                    payload: "{}".into(),
                    origin: RowOrigin::Live,
                    state: OutboxState::Pending,
                    last_class: None,
                }],
                &[],
                now,
            )
            .unwrap();
        let rows = store.claim_ready(SyncService::Inara, None, 1, now).unwrap();
        store.mark_sent(&[rows[0].id], "200", now).unwrap();

        agg.on_worker_event(&WorkerEvent::Outcome {
            service: SyncService::Inara,
            cmdr_fid: F1.into(),
            effect: ServiceEffect::Connected,
            sent: 1,
            rejected: 0,
        });
        let after = agg.build(Some(F1), &|_, _| None);
        let st = &inara_status(&after).status;
        assert_eq!(st.state, SyncState::Connected);
        assert_eq!(st.sent_total, 1);
        assert_eq!(st.sent_session, 1);
        assert!(st.last_confirmed_at.is_some());
    }

    #[test]
    fn error_shows_class_and_next_attempt() {
        let (store, clock, mut agg) = setup();
        let now = clock.now_utc();
        store
            .enqueue_batch_with_cursor(
                &[NewOutboxRow {
                    service: SyncService::Inara,
                    cmdr_fid: F1.into(),
                    idem_key: "k-net".into(),
                    kind: "FSDJump".into(),
                    coalesce_key: None,
                    event_ts: now,
                    payload: "{}".into(),
                    origin: RowOrigin::Live,
                    state: OutboxState::Pending,
                    last_class: None,
                }],
                &[],
                now,
            )
            .unwrap();
        let rows = store.claim_ready(SyncService::Inara, None, 1, now).unwrap();
        let next = now + chrono::Duration::seconds(60);
        store
            .schedule_retry(&[rows[0].id], next, "net", ErrorClass::Network)
            .unwrap();
        agg.on_worker_event(&WorkerEvent::Outcome {
            service: SyncService::Inara,
            cmdr_fid: F1.into(),
            effect: ServiceEffect::ServiceError {
                class: ErrorClass::Network,
                until: None,
            },
            sent: 0,
            rejected: 0,
        });
        let reports = agg.build(Some(F1), &|_, _| None);
        let st = &inara_status(&reports).status;
        assert_eq!(st.state, SyncState::Error);
        assert_eq!(st.reason, Some(SyncReason::Network));
        assert!(st.next_attempt_at.is_some());
    }

    #[tokio::test(start_paused = true)]
    async fn reports_rate_limited_to_one_per_second_and_only_on_change() {
        let (_store, _clock, mut agg) = setup();
        let t0 = tokio::time::Instant::now();
        let r0 = agg.build(Some(F1), &|_, _| None);
        assert_eq!(agg.changed_reports(r0.clone(), t0).len(), 4);
        for ms in [100u64, 200, 300, 400] {
            let emitted = agg.changed_reports(r0.clone(), t0 + Duration::from_millis(ms));
            assert!(emitted.is_empty());
        }

        // Cambio en el +0,5 s: queda retenido hasta cumplir el segundo completo.
        agg.on_recovered(SyncService::Inara, 2);
        let r1 = agg.build(Some(F1), &|_, _| None);
        let at_half = agg.changed_reports(r1.clone(), t0 + Duration::from_millis(500));
        assert!(at_half.is_empty());

        let at_one = agg.changed_reports(r1, t0 + Duration::from_secs(1));
        assert_eq!(at_one.len(), 1);
        assert_eq!(inara_status(&at_one).status.recovered_session, 2);
    }

    #[test]
    fn recent_rejections_capped_at_20_and_hint_trimmed() {
        let (_store, clock, mut agg) = setup();
        let at = clock.now_utc().to_rfc3339_opts(SecondsFormat::Secs, true);
        let long_hint = "x".repeat(300);
        for i in 0..25 {
            agg.on_worker_event(&WorkerEvent::Rejection {
                service: SyncService::Inara,
                cmdr_fid: F1.into(),
                rejection: RecentRejection {
                    kind: format!("Evt{i}"),
                    class: ErrorClass::InvalidEvent,
                    code: "400".into(),
                    at: at.clone(),
                    hint: Some(long_hint.clone()),
                },
            });
        }
        let reports = agg.build(Some(F1), &|_, _| None);
        let rejections = &inara_status(&reports).status.recent_rejections;
        assert_eq!(rejections.len(), 20);
        assert_eq!(rejections[0].kind, "Evt24");
        assert_eq!(rejections[0].hint.as_deref().unwrap().len(), 120);
    }

    #[test]
    fn roster_counts_other_cmdrs_pending() {
        let (store, clock, agg) = setup();
        let now = clock.now_utc();
        store.upsert_known_cmdr(F2, "CMDR BETA", now).unwrap();
        let mk = |fid: &str, key: &str| NewOutboxRow {
            service: SyncService::Inara,
            cmdr_fid: fid.into(),
            idem_key: key.into(),
            kind: "FSDJump".into(),
            coalesce_key: None,
            event_ts: now,
            payload: "{}".into(),
            origin: RowOrigin::Live,
            state: OutboxState::Pending,
            last_class: None,
        };
        store
            .enqueue_batch_with_cursor(
                &[
                    mk(F1, "a1"),
                    mk(F1, "a2"),
                    mk(F2, "b1"),
                    mk(F2, "b2"),
                    mk(F2, "b3"),
                ],
                &[],
                now,
            )
            .unwrap();

        let reports = agg.build(Some(F1), &|_, _| None);
        let roster = reports
            .iter()
            .find_map(|r| match r {
                SyncReport::Roster(r) => Some(r),
                _ => None,
            })
            .unwrap();
        assert_eq!(roster.other_cmdrs_pending.inara, 3);
        assert_eq!(roster.current_cmdr.as_ref().unwrap().name, "CMDR ALPHA");
    }
}
