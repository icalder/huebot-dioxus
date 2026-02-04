use crate::hue::events::HueEvent;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MotionData {
    pub id: String,
    pub id_v1: Option<String>,
    pub enabled: bool,
    pub presence: bool,
    pub last_updated: DateTime<Utc>,
    #[serde(default)]
    pub history: Arc<Vec<(DateTime<Utc>, bool)>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemperatureData {
    pub id: String,
    pub id_v1: Option<String>,
    pub enabled: bool,
    pub temperature: f64,
    pub last_updated: DateTime<Utc>,
    #[serde(default)]
    pub history: Arc<Vec<(DateTime<Utc>, f64)>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LightData {
    pub id: String,
    pub id_v1: Option<String>,
    pub enabled: bool,
    pub light_level: i32,
    pub last_updated: DateTime<Utc>,
    #[serde(default)]
    pub history: Arc<Vec<(DateTime<Utc>, i32)>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeSensor {
    pub device_id: String,
    pub name: String,
    pub is_outdoor: bool,
    pub enabled: bool,
    pub motion: Option<MotionData>,
    pub temperature: Option<TemperatureData>,
    pub light: Option<LightData>,
}

impl CompositeSensor {
    pub fn apply_event(&mut self, event: &HueEvent) {
        match event {
            HueEvent::Motion {
                id,
                presence,
                changed,
                enabled,
                ..
            } => {
                let motion = self.motion.get_or_insert_with(|| MotionData {
                    id: id.clone(),
                    id_v1: None,
                    enabled: *enabled,
                    presence: *presence,
                    last_updated: *changed,
                    history: Arc::new(Vec::new()),
                });

                motion.enabled = *enabled;
                motion.presence = *presence;
                motion.last_updated = *changed;
                Self::update_history(Arc::make_mut(&mut motion.history), *changed, *presence);
            }
            HueEvent::Temperature {
                id,
                temperature,
                changed,
                enabled,
                ..
            } => {
                let temp = self.temperature.get_or_insert_with(|| TemperatureData {
                    id: id.clone(),
                    id_v1: None,
                    enabled: *enabled,
                    temperature: *temperature,
                    last_updated: *changed,
                    history: Arc::new(Vec::new()),
                });

                temp.enabled = *enabled;
                temp.temperature = *temperature;
                temp.last_updated = *changed;
                Self::update_history(Arc::make_mut(&mut temp.history), *changed, *temperature);
            }
            HueEvent::LightLevel {
                id,
                light_level,
                changed,
                enabled,
                ..
            } => {
                let light = self.light.get_or_insert_with(|| LightData {
                    id: id.clone(),
                    id_v1: None,
                    enabled: *enabled,
                    light_level: *light_level,
                    last_updated: *changed,
                    history: Arc::new(Vec::new()),
                });

                light.enabled = *enabled;
                light.light_level = *light_level;
                light.last_updated = *changed;
                Self::update_history(Arc::make_mut(&mut light.history), *changed, *light_level);
            }
            HueEvent::Raw(_) => return,
        }

        self.enabled = self.motion.as_ref().map(|m| m.enabled).unwrap_or(true)
            && self.temperature.as_ref().map(|t| t.enabled).unwrap_or(true)
            && self.light.as_ref().map(|l| l.enabled).unwrap_or(true);
    }

    pub fn fingerprint(&self) -> String {
        format!(
            "{}-{}-{}",
            self.motion
                .as_ref()
                .map(|m| m.last_updated.to_rfc3339())
                .unwrap_or_default(),
            self.temperature
                .as_ref()
                .map(|t| t.last_updated.to_rfc3339())
                .unwrap_or_default(),
            self.light
                .as_ref()
                .map(|l| l.last_updated.to_rfc3339())
                .unwrap_or_default(),
        )
    }

    pub fn update_history<T: PartialEq + Clone + std::fmt::Display>(
        history: &mut Vec<(DateTime<Utc>, T)>,
        time: DateTime<Utc>,
        val: T,
    ) {
        if !history.iter().any(|(t, _)| *t == time) {
            history.push((time, val));
        }
        history.sort_by_key(|(t, _)| *t);

        let limit = Utc::now() - chrono::Duration::minutes(15);

        // Find the index of the first point that is within the limit
        let cutoff_index = history.iter().position(|(t, _)| *t >= limit);

        if let Some(idx) = cutoff_index {
            // If there are points older than the limit, we want to keep the one immediately preceding the cutoff
            // as an anchor for the graph rendering (so we know the value at the start of the window).
            // So we remove everything before (idx - 1).
            if idx > 0 {
                history.drain(0..(idx - 1));
            }
        } else {
            // All points are older than the limit.
            // Keep only the last one (most recent state).
            if history.len() > 1 {
                let keep = history.len() - 1;
                history.drain(0..keep);
            }
        }
    }
}
