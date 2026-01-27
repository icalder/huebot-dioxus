use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::hue::events::HueEvent;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MotionData {
    pub id: String,
    pub id_v1: Option<String>,
    pub enabled: bool,
    pub presence: bool,
    pub last_updated: DateTime<Utc>,
    #[serde(default)]
    pub history: Vec<(DateTime<Utc>, bool)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemperatureData {
    pub id: String,
    pub id_v1: Option<String>,
    pub enabled: bool,
    pub temperature: f64,
    pub last_updated: DateTime<Utc>,
    #[serde(default)]
    pub history: Vec<(DateTime<Utc>, f64)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LightData {
    pub id: String,
    pub id_v1: Option<String>,
    pub enabled: bool,
    pub light_level: i32,
    pub last_updated: DateTime<Utc>,
    #[serde(default)]
    pub history: Vec<(DateTime<Utc>, i32)>,
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
            HueEvent::Motion { id, presence, changed, enabled, .. } => {
                let mut motion = MotionData {
                    id: id.clone(),
                    id_v1: self.motion.as_ref().and_then(|m| m.id_v1.clone()),
                    enabled: *enabled,
                    presence: *presence,
                    last_updated: *changed,
                    history: self.motion.as_ref().map(|m| m.history.clone()).unwrap_or_default(),
                };
                Self::update_history(&mut motion.history, *changed, *presence);
                self.motion = Some(motion);
            }
            HueEvent::Temperature { id, temperature, changed, enabled, .. } => {
                let mut temp = TemperatureData {
                    id: id.clone(),
                    id_v1: self.temperature.as_ref().and_then(|t| t.id_v1.clone()),
                    enabled: *enabled,
                    temperature: *temperature,
                    last_updated: *changed,
                    history: self.temperature.as_ref().map(|t| t.history.clone()).unwrap_or_default(),
                };
                Self::update_history(&mut temp.history, *changed, *temperature);
                self.temperature = Some(temp);
            }
            HueEvent::LightLevel { id, light_level, changed, enabled, .. } => {
                let mut light = LightData {
                    id: id.clone(),
                    id_v1: self.light.as_ref().and_then(|l| l.id_v1.clone()),
                    enabled: *enabled,
                    light_level: *light_level,
                    last_updated: *changed,
                    history: self.light.as_ref().map(|l| l.history.clone()).unwrap_or_default(),
                };
                Self::update_history(&mut light.history, *changed, *light_level);
                self.light = Some(light);
            }
            HueEvent::Raw(_) => return,
        }

        self.enabled = self.motion.as_ref().map(|m| m.enabled).unwrap_or(true)
            && self.temperature.as_ref().map(|t| t.enabled).unwrap_or(true)
            && self.light.as_ref().map(|l| l.enabled).unwrap_or(true);
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
