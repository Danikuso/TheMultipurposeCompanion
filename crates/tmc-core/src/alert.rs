use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
pub struct Alert {
    pub id: String,
    pub severity: Severity,
    pub key: String,
    pub args: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warn,
    Critical,
}

impl Alert {
    pub fn new(severity: Severity, key: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            severity,
            key: key.into(),
            args: BTreeMap::new(),
        }
    }

    pub fn arg(mut self, name: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.args.insert(name.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alert_serializes_fluent_key_and_typed_arguments() {
        let alert = Alert::new(Severity::Critical, "alert-hull-damage")
            .arg("pct", 25)
            .arg("module", "dashboard");
        let json = serde_json::to_value(alert).unwrap();
        assert_eq!(json["severity"], "critical");
        assert_eq!(json["key"], "alert-hull-damage");
        assert_eq!(
            json["args"],
            serde_json::json!({"pct": 25, "module": "dashboard"})
        );
        assert_eq!(serde_json::to_value(Severity::Info).unwrap(), "info");
        assert_eq!(serde_json::to_value(Severity::Warn).unwrap(), "warn");
    }

    #[test]
    fn alert_ids_are_distinct_uuid_v4() {
        let first = Alert::new(Severity::Warn, "alert-low-fuel");
        let second = Alert::new(Severity::Warn, "alert-low-fuel");
        let parsed = uuid::Uuid::parse_str(&first.id).unwrap();
        assert_eq!(parsed.get_version_num(), 4);
        assert_ne!(first.id, second.id);
        assert!(first.args.is_empty());
    }
}
