use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum HueEvent {
    Motion {
        id: String,
        owner_rid: String,
        presence: bool,
        changed: DateTime<Utc>,
        enabled: bool,
    },
    Temperature {
        id: String,
        owner_rid: String,
        temperature: f64,
        changed: DateTime<Utc>,
        enabled: bool,
    },
    LightLevel {
        id: String,
        owner_rid: String,
        light_level: i32,
        changed: DateTime<Utc>,
        enabled: bool,
    },
    Raw(serde_json::Value),
}

impl HueEvent {
    pub fn from_json(v: &serde_json::Value) -> Option<Self> {
        let event_type = v.get("type").and_then(|t| t.as_str());
        let id = v.get("id").and_then(|id| id.as_str());

        if let (Some(t), Some(id)) = (event_type, id) {
            let id = id.to_string();
            let owner_rid = v
                .get("owner")
                .and_then(|o| o.get("rid"))
                .and_then(|rid| rid.as_str())
                .unwrap_or_default()
                .to_string();
            let enabled = v.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true);

            match t {
                "motion" => {
                    if let Some(report) = v.get("motion").and_then(|m| m.get("motion_report")) {
                        let presence = report.get("motion").and_then(|m| m.as_bool()).unwrap_or(false);
                        let changed = crate::hue::client::ClientEx::parse_date(
                            &report.get("changed").and_then(|c| c.as_str()).map(|s| s.to_string()),
                        );
                        return Some(Self::Motion {
                            id,
                            owner_rid,
                            presence,
                            changed,
                            enabled,
                        });
                    }
                }
                "temperature" => {
                    if let Some(report) = v.get("temperature").and_then(|t| t.get("temperature_report")) {
                        let temperature = report.get("temperature").and_then(|t| t.as_f64()).unwrap_or(0.0);
                        let changed = crate::hue::client::ClientEx::parse_date(
                            &report.get("changed").and_then(|c| c.as_str()).map(|s| s.to_string()),
                        );
                        return Some(Self::Temperature {
                            id,
                            owner_rid,
                            temperature,
                            changed,
                            enabled,
                        });
                    }
                }
                "light_level" => {
                    if let Some(report) = v.get("light").and_then(|l| l.get("light_level_report")) {
                        let light_level = report.get("light_level").and_then(|l| l.as_i64()).unwrap_or(0) as i32;
                        let changed = crate::hue::client::ClientEx::parse_date(
                            &report.get("changed").and_then(|c| c.as_str()).map(|s| s.to_string()),
                        );
                        return Some(Self::LightLevel {
                            id,
                            owner_rid,
                            light_level,
                            changed,
                            enabled,
                        });
                    }
                }
                _ => {}
            }
        }
        
        if v.is_object() {
            Some(Self::Raw(v.clone()))
        } else {
            None
        }
    }

    pub fn owner_rid(&self) -> Option<&str> {
        match self {
            Self::Motion { owner_rid, .. } => Some(owner_rid),
            Self::Temperature { owner_rid, .. } => Some(owner_rid),
            Self::LightLevel { owner_rid, .. } => Some(owner_rid),
            Self::Raw(v) => v
                .get("owner")
                .and_then(|o| o.get("rid"))
                .and_then(|rid| rid.as_str()),
        }
    }

    pub fn resource_id(&self) -> Option<&str> {
        match self {
            Self::Motion { id, .. } => Some(id),
            Self::Temperature { id, .. } => Some(id),
            Self::LightLevel { id, .. } => Some(id),
            Self::Raw(v) => v.get("id").and_then(|id| id.as_str()),
        }
    }
}





