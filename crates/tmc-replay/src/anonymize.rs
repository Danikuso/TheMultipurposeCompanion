//! Anonimización de líneas del diario de Elite Dangerous.

use serde_json::{Map, Value};
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone)]
pub struct Pseudonyms {
    pub names: BTreeMap<String, String>,
    pub fids: BTreeMap<String, String>,
    pub ship_names: BTreeMap<String, String>,
    pub ship_idents: BTreeMap<String, String>,
    pub squadrons: BTreeMap<String, String>,
}

impl Pseudonyms {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn name(&mut self, original: &str) -> String {
        if original.is_empty() {
            return String::new();
        }
        if let Some(existing) = self.names.get(original) {
            return existing.clone();
        }
        let replacement = if self.names.is_empty() {
            "CMDR TESTER".to_owned()
        } else {
            format!("CMDR_{:02}", self.names.len() + 1)
        };
        self.names.insert(original.to_owned(), replacement.clone());
        replacement
    }

    pub fn fid(&mut self, original: &str) -> String {
        if original.is_empty() {
            return String::new();
        }
        if let Some(existing) = self.fids.get(original) {
            return existing.clone();
        }
        let replacement = format!("F{:07}", self.fids.len() + 1);
        self.fids.insert(original.to_owned(), replacement.clone());
        replacement
    }

    pub fn ship_name(&mut self, original: &str) -> String {
        if original.is_empty() {
            return String::new();
        }
        if let Some(existing) = self.ship_names.get(original) {
            return existing.clone();
        }
        let replacement = format!("TMC VESSEL {:02}", self.ship_names.len() + 1);
        self.ship_names
            .insert(original.to_owned(), replacement.clone());
        replacement
    }

    pub fn ship_ident(&mut self, original: &str) -> String {
        if original.is_empty() {
            return String::new();
        }
        if let Some(existing) = self.ship_idents.get(original) {
            return existing.clone();
        }
        let replacement = format!("TMC-{:02}", self.ship_idents.len() + 1);
        self.ship_idents
            .insert(original.to_owned(), replacement.clone());
        replacement
    }

    pub fn squadron(&mut self, original: &str) -> String {
        if original.is_empty() {
            return String::new();
        }
        if let Some(existing) = self.squadrons.get(original) {
            return existing.clone();
        }
        let replacement = format!("SQUADRON {:02}", self.squadrons.len() + 1);
        self.squadrons
            .insert(original.to_owned(), replacement.clone());
        replacement
    }
}

fn replace_string_field<F>(obj: &mut Map<String, Value>, key: &str, mut f: F)
where
    F: FnMut(&str) -> String,
{
    if let Some(Value::String(s)) = obj.get(key) {
        let new_val = f(s);
        obj.insert(key.to_owned(), Value::String(new_val));
    }
}

fn replace_case_insensitive(haystack: &str, needle: &str, replacement: &str) -> String {
    if needle.is_empty() {
        return haystack.to_owned();
    }
    let lower_h = haystack.to_ascii_lowercase();
    let lower_n = needle.to_ascii_lowercase();
    if !lower_h.contains(&lower_n) {
        return haystack.to_owned();
    }
    let mut result = String::with_capacity(haystack.len());
    let mut last_idx = 0;
    for (idx, _) in lower_h.match_indices(&lower_n) {
        result.push_str(&haystack[last_idx..idx]);
        result.push_str(replacement);
        last_idx = idx + needle.len();
    }
    result.push_str(&haystack[last_idx..]);
    result
}

fn scrub_known_tokens(val: &mut Value, map: &Pseudonyms) {
    match val {
        Value::String(s) => {
            for (real, pseudo) in &map.names {
                if !real.is_empty() {
                    *s = replace_case_insensitive(s, real, pseudo);
                }
            }
            for (real, pseudo) in &map.fids {
                if !real.is_empty() {
                    *s = replace_case_insensitive(s, real, pseudo);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                scrub_known_tokens(item, map);
            }
        }
        Value::Object(obj) => {
            for (_, v) in obj.iter_mut() {
                scrub_known_tokens(v, map);
            }
        }
        _ => {}
    }
}

pub fn anonymize_line(line: &str, map: &mut Pseudonyms) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut root: Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(_) => return Some(trimmed.to_owned()),
    };
    let Some(obj) = root.as_object_mut() else {
        return Some(trimmed.to_owned());
    };

    let event = obj
        .get("event")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();

    if event == "Friends" {
        return None;
    }

    replace_string_field(obj, "FID", |s| map.fid(s));
    replace_string_field(obj, "Commander", |s| map.name(s));
    replace_string_field(obj, "ShipName", |s| map.ship_name(s));
    replace_string_field(obj, "UserShipName", |s| map.ship_name(s));
    replace_string_field(obj, "ShipIdent", |s| map.ship_ident(s));
    replace_string_field(obj, "UserShipId", |s| map.ship_ident(s));
    replace_string_field(obj, "SquadronName", |s| map.squadron(s));

    match event.as_str() {
        "Commander"
        | "NewCommander"
        | "ClearSavedGame"
        | "WingAdd"
        | "WingInvite"
        | "CrewMemberJoins"
        | "CrewMemberQuits"
        | "CrewLaunchFighter"
        | "CrewMemberRoleChange"
        | "KickCrewMember" => {
            replace_string_field(obj, "Name", |s| map.name(s));
        }
        "JoinACrew" | "QuitACrew" => {
            replace_string_field(obj, "Captain", |s| map.name(s));
        }
        "WingJoin" => {
            if let Some(Value::Array(others)) = obj.get_mut("Others") {
                for item in others {
                    if let Value::String(s) = item {
                        *s = map.name(s);
                    }
                }
            }
        }
        "StoredShips" => {
            for key in ["ShipsHere", "ShipsRemote"] {
                if let Some(Value::Array(ships)) = obj.get_mut(key) {
                    for ship in ships {
                        if let Some(ship_obj) = ship.as_object_mut() {
                            replace_string_field(ship_obj, "Name", |s| map.ship_name(s));
                        }
                    }
                }
            }
        }
        "ReceiveText" | "SendText" => {
            let is_npc = obj
                .get("Channel")
                .and_then(|v| v.as_str())
                .is_some_and(|c| c.eq_ignore_ascii_case("npc"));
            if !is_npc {
                replace_string_field(obj, "From", |s| map.name(s));
                replace_string_field(obj, "From_Localised", |s| map.name(s));
            }
            replace_string_field(obj, "To", |s| map.name(s));
            replace_string_field(obj, "To_Localised", |s| map.name(s));
            if obj.contains_key("Message") {
                obj.insert("Message".to_owned(), Value::String("[REDACTED]".to_owned()));
            }
            if obj.contains_key("Message_Localised") {
                obj.insert(
                    "Message_Localised".to_owned(),
                    Value::String("[REDACTED]".to_owned()),
                );
            }
        }
        _ => {}
    }

    scrub_known_tokens(&mut root, map);
    serde_json::to_string(&root).ok()
}
