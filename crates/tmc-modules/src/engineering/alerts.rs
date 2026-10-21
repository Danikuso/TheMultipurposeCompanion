//! Avisos de ingeniería por transición (spec §3.8): un aviso solo salta cuando
//! cambia el estado, nunca por estar en él.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use tmc_model::events::EngineerStatus;

use super::engineers::EngineerStatusChange;
use super::goals::ShoppingView;
use super::inventory::MaterialChange;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum EngAlertKind {
    MaterialAtMax {
        key: String,
    },
    GoalCompletable {
        goal: String,
        engineer: Option<String>,
    },
    EngineerUnlocked {
        id: u64,
    },
    EngineerInvited {
        id: u64,
    },
    ScarceObtained {
        key: String,
        goal: String,
    },
}

/// Blueprint y grado objetivo del id de un objetivo
/// (`"{ship}:{slot}:{blueprint}:{experimental|-}:{grade}"`).
pub fn goal_blueprint(goal_id: &str) -> Option<(&str, u8)> {
    let parts: Vec<&str> = goal_id.split(':').collect();
    let [_, _, blueprint, _, grade] = parts[..] else {
        return None;
    };
    Some((blueprint, grade.parse().ok()?))
}

/// Avisos por transición. `engineer_for(blueprint, grade)` da el nombre del ingeniero
/// recomendado (p. ej. con `pick_engineer`) para citarlo en `GoalCompletable`.
/// `ScarceObtained`: se obtuvo un material que faltaba antes (`missing_before`);
/// solo salta cuando ningún objetivo lo echa ya en falta; `goal` es el primero de la lista.
#[allow(clippy::too_many_arguments)] // firma fijada por el plan (+ `engineer_for`)
pub fn alerts_from(
    changes: &[MaterialChange],
    at_max_before: &BTreeSet<String>,
    at_max_now: &BTreeSet<String>,
    completable_before: &BTreeSet<String>,
    view: &ShoppingView,
    status: &[EngineerStatusChange],
    missing_before: &BTreeMap<String, u32>,
    engineer_for: &dyn Fn(&str, u8) -> Option<String>,
) -> Vec<EngAlertKind> {
    let mut out = Vec::new();
    for key in at_max_now.difference(at_max_before) {
        out.push(EngAlertKind::MaterialAtMax { key: key.clone() });
    }
    for g in view
        .goals
        .iter()
        .filter(|g| g.completable && !completable_before.contains(&g.id))
    {
        let engineer = goal_blueprint(&g.id).and_then(|(bp, grade)| engineer_for(bp, grade));
        out.push(EngAlertKind::GoalCompletable {
            goal: g.id.clone(),
            engineer,
        });
    }
    for c in status {
        if c.from == Some(c.to) {
            continue;
        }
        match c.to {
            EngineerStatus::Unlocked => out.push(EngAlertKind::EngineerUnlocked { id: c.id }),
            EngineerStatus::Invited => out.push(EngAlertKind::EngineerInvited { id: c.id }),
            _ => {}
        }
    }
    let mut seen = BTreeSet::new();
    for c in changes.iter().filter(|c| c.delta > 0) {
        if missing_before.get(&c.key).copied().unwrap_or(0) == 0 || !seen.insert(&c.key) {
            continue;
        }
        // Solo cuando el faltante de ese material se cubre del todo (no por unidad).
        if view.goals.iter().any(|g| g.missing.contains_key(&c.key)) {
            continue;
        }
        let goal = view.goals.first();
        if let Some(g) = goal {
            out.push(EngAlertKind::ScarceObtained {
                key: c.key.clone(),
                goal: g.id.clone(),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engineering::goals::GoalView;
    use crate::engineering::inventory::ChangeOrigin;
    use chrono::Utc;

    fn set(keys: &[&str]) -> BTreeSet<String> {
        keys.iter().map(|k| (*k).to_string()).collect()
    }

    fn change(key: &str, delta: i32) -> MaterialChange {
        MaterialChange {
            key: key.into(),
            delta,
            origin: ChangeOrigin::Collected,
            lost: 0,
            at: Utc::now(),
        }
    }

    fn goal(id: &str, missing: &[(&str, u32)], completable: bool) -> GoalView {
        GoalView {
            id: id.into(),
            percent: 0,
            missing: missing.iter().map(|(k, n)| ((*k).into(), *n)).collect(),
            trades: Vec::new(),
            completable,
        }
    }

    fn view(goals: Vec<GoalView>) -> ShoppingView {
        ShoppingView {
            status: "ok".into(),
            goals,
            needs: BTreeMap::new(),
            missing: BTreeMap::new(),
            surplus: BTreeMap::new(),
        }
    }

    fn run(
        changes: &[MaterialChange],
        max: (&BTreeSet<String>, &BTreeSet<String>),
        completable_before: &BTreeSet<String>,
        v: &ShoppingView,
        status: &[EngineerStatusChange],
        missing_before: &BTreeMap<String, u32>,
    ) -> Vec<EngAlertKind> {
        alerts_from(
            changes,
            max.0,
            max.1,
            completable_before,
            v,
            status,
            missing_before,
            &|bp, g| (bp == "fsd_longrange" && g <= 5).then(|| "Felicity Farseer".to_string()),
        )
    }

    #[test]
    fn alert_only_on_transition() {
        let v = view(Vec::new());
        let none = BTreeSet::new();
        let empty = BTreeMap::new();
        let iron = set(&["iron"]);
        assert!(run(&[], (&iron, &iron), &none, &v, &[], &empty).is_empty());
        assert_eq!(
            run(&[], (&none, &iron), &none, &v, &[], &empty),
            vec![EngAlertKind::MaterialAtMax { key: "iron".into() }]
        );
        let status = [
            EngineerStatusChange {
                id: 300160,
                from: Some(EngineerStatus::Known),
                to: EngineerStatus::Invited,
            },
            EngineerStatusChange {
                id: 300100,
                from: None,
                to: EngineerStatus::Unlocked,
            },
            EngineerStatusChange {
                id: 300220,
                from: Some(EngineerStatus::Known),
                to: EngineerStatus::Acquainted,
            },
        ];
        assert_eq!(
            run(&[], (&none, &none), &none, &v, &status, &empty),
            vec![
                EngAlertKind::EngineerInvited { id: 300160 },
                EngAlertKind::EngineerUnlocked { id: 300100 }
            ]
        );
    }

    #[test]
    fn goal_completable_alert_names_nearby_engineer() {
        let id = "1:fsd:fsd_longrange:-:5";
        let v = view(vec![goal(id, &[], true), goal("1:x:other:-:2", &[], false)]);
        let none = BTreeSet::new();
        let empty = BTreeMap::new();
        assert_eq!(
            run(&[], (&none, &none), &none, &v, &[], &empty),
            vec![EngAlertKind::GoalCompletable {
                goal: id.into(),
                engineer: Some("Felicity Farseer".into())
            }]
        );
        // ya completable antes: sin aviso
        assert!(run(&[], (&none, &none), &set(&[id]), &v, &[], &empty).is_empty());
        assert_eq!(goal_blueprint("garbage"), None);
        assert_eq!(goal_blueprint(id), Some(("fsd_longrange", 5)));
    }

    #[test]
    fn scarce_material_obtained_alert_only_when_shortfall_reaches_zero() {
        let id = "1:fsd:fsd_longrange:-:5";
        let none = BTreeSet::new();
        let before = BTreeMap::from([("iron".to_string(), 2)]);
        // Aún falta 1: sin aviso (no es por unidad).
        let still = view(vec![goal(id, &[("iron", 1)], false)]);
        assert!(
            run(
                &[change("iron", 1)],
                (&none, &none),
                &none,
                &still,
                &[],
                &before
            )
            .is_empty()
        );
        // Cubierto del todo: un único aviso.
        let covered = view(vec![goal(id, &[], false)]);
        assert_eq!(
            run(
                &[change("iron", 1), change("iron", 1)],
                (&none, &none),
                &none,
                &covered,
                &[],
                &before
            ),
            vec![EngAlertKind::ScarceObtained {
                key: "iron".into(),
                goal: id.into()
            }]
        );
        // material que no faltaba o pérdida: sin aviso
        assert!(
            run(
                &[change("nickel", 1)],
                (&none, &none),
                &none,
                &covered,
                &[],
                &before
            )
            .is_empty()
        );
        assert!(
            run(
                &[change("iron", -1)],
                (&none, &none),
                &none,
                &covered,
                &[],
                &before
            )
            .is_empty()
        );
    }
}
