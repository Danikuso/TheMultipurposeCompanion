//! Mapeo Inara de rango, reputación y créditos del comandante.

use serde_json::{Map, Value, json};
use tmc_model::events::{EngineerStatus, EventKind, LoadGame, Rank};

use super::{inara_row, raw_value, ts_iso};
use crate::router::{MappedInput, MappedRow};

pub(crate) fn map_commander(fid: &str, input: &MappedInput<'_>, out: &mut Vec<MappedRow>) {
    let ts_owned = ts_iso(input);
    let ts = ts_owned.as_str();

    match &input.ev.kind {
        EventKind::LoadGame(lg) => {
            push_credits(fid, ts, lg, out);
        }
        EventKind::Rank(_) | EventKind::Progress(_) => {
            push_pilot_ranks(
                fid,
                ts,
                input.snap.ranks.as_ref(),
                input.snap.rank_progress.as_ref(),
                out,
            );
        }
        EventKind::EngineerProgress(ep) => {
            let entries: Vec<Value> = ep
                .entries()
                .into_iter()
                .filter_map(|e| {
                    let stage = match e.progress {
                        EngineerStatus::Invited => "Invited",
                        EngineerStatus::Acquainted => "Acquainted",
                        EngineerStatus::Unlocked => "Unlocked",
                        EngineerStatus::Barred => "Barred",
                        EngineerStatus::Known | EngineerStatus::Unknown => return None,
                    };
                    let mut m = Map::new();
                    m.insert("engineerName".into(), json!(e.engineer));
                    m.insert("rankStage".into(), json!(stage));
                    if let Some(rank) = e.rank {
                        m.insert("rankValue".into(), json!(rank));
                    }
                    Some(Value::Object(m))
                })
                .collect();
            if !entries.is_empty() {
                out.push(inara_row(
                    fid,
                    "setCommanderRankEngineer",
                    0,
                    ts,
                    Value::Array(entries),
                    Some("all"),
                ));
            }
        }
        EventKind::Powerplay(pp) if !pp.power.trim().is_empty() => {
            out.push(inara_row(
                fid,
                "setCommanderRankPower",
                0,
                ts,
                json!({
                    "powerName": pp.power,
                    "rankValue": pp.rank,
                    "meritsValue": pp.merits,
                }),
                Some("power"),
            ));
        }
        EventKind::Other if input.ev.name == "Reputation" => {
            if let Some(raw) = raw_value(input) {
                push_major_factions(fid, ts, &raw, out);
            }
        }
        _ => {}
    }
}

fn push_credits(fid: &str, ts: &str, lg: &LoadGame, out: &mut Vec<MappedRow>) {
    let Some(credits) = lg.credits else {
        return;
    };
    let mut data = Map::new();
    data.insert("commanderCredits".into(), json!(credits));
    if let Some(loan) = lg.loan {
        data.insert("commanderLoan".into(), json!(loan));
    }
    out.push(inara_row(
        fid,
        "setCommanderCredits",
        0,
        ts,
        Value::Object(data),
        Some("credits"),
    ));
}

pub(crate) fn push_pilot_ranks(
    fid: &str,
    ts: &str,
    ranks: Option<&Rank>,
    progress: Option<&Rank>,
    out: &mut Vec<MappedRow>,
) {
    let specs: [(&str, Option<u32>, Option<u32>); 8] = [
        (
            "combat",
            ranks.and_then(|r| r.combat),
            progress.and_then(|p| p.combat),
        ),
        (
            "trade",
            ranks.and_then(|r| r.trade),
            progress.and_then(|p| p.trade),
        ),
        (
            "explore",
            ranks.and_then(|r| r.explore),
            progress.and_then(|p| p.explore),
        ),
        (
            "soldier",
            ranks.and_then(|r| r.soldier),
            progress.and_then(|p| p.soldier),
        ),
        (
            "exobiologist",
            ranks.and_then(|r| r.exobiologist),
            progress.and_then(|p| p.exobiologist),
        ),
        (
            "empire",
            ranks.and_then(|r| r.empire),
            progress.and_then(|p| p.empire),
        ),
        (
            "federation",
            ranks.and_then(|r| r.federation),
            progress.and_then(|p| p.federation),
        ),
        (
            "cqc",
            ranks.and_then(|r| r.cqc),
            progress.and_then(|p| p.cqc),
        ),
    ];

    let entries: Vec<Value> = specs
        .into_iter()
        .filter_map(|(name, rv, rp)| {
            if rv.is_none() && rp.is_none() {
                return None;
            }
            let mut m = Map::new();
            m.insert("rankName".into(), json!(name));
            if let Some(v) = rv {
                m.insert("rankValue".into(), json!(v));
            }
            if let Some(p) = rp {
                m.insert("rankProgress".into(), json!(f64::from(p) / 100.0));
            }
            Some(Value::Object(m))
        })
        .collect();

    if !entries.is_empty() {
        out.push(inara_row(
            fid,
            "setCommanderRankPilot",
            0,
            ts,
            Value::Array(entries),
            Some("pilot"),
        ));
    }
}

fn push_major_factions(fid: &str, ts: &str, raw: &Value, out: &mut Vec<MappedRow>) {
    let keys = [
        ("Empire", "empire"),
        ("Federation", "federation"),
        ("Alliance", "alliance"),
        ("Independent", "independent"),
    ];
    let entries: Vec<Value> = keys
        .into_iter()
        .filter_map(|(journal_key, inara_key)| {
            raw.get(journal_key).and_then(Value::as_f64).map(|v| {
                json!({
                    "majorfactionName": inara_key,
                    "majorfactionReputation": v / 100.0,
                })
            })
        })
        .collect();

    if !entries.is_empty() {
        out.push(inara_row(
            fid,
            "setCommanderReputationMajorFaction",
            0,
            ts,
            Value::Array(entries),
            Some("major"),
        ));
    }
}

pub(crate) fn push_minor_factions(
    fid: &str,
    ts: &str,
    raw: &Value,
    start_sub: u32,
    out: &mut Vec<MappedRow>,
) {
    let Some(factions) = raw.get("Factions").and_then(Value::as_array) else {
        return;
    };
    let mut sub = start_sub;
    for f in factions {
        let Some(name) = f
            .get("Name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            continue;
        };
        let Some(rep) = f.get("MyReputation").and_then(Value::as_f64) else {
            continue;
        };
        out.push(inara_row(
            fid,
            "setCommanderReputationMinorFaction",
            sub,
            ts,
            json!({
                "minorfactionName": name,
                "minorfactionReputation": rep / 100.0,
            }),
            Some(name),
        ));
        sub += 1;
    }
}
