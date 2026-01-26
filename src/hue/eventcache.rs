#[cfg(feature = "server")]
use chrono::{DateTime, Duration, Utc};
#[cfg(feature = "server")]
use std::collections::VecDeque;
#[cfg(feature = "server")]
use std::sync::RwLock;

#[cfg(feature = "server")]
pub struct EventCache {
    events: RwLock<VecDeque<(DateTime<Utc>, String)>>,
    max_age: Duration,
}

#[cfg(feature = "server")]
impl EventCache {
    pub fn new(max_age_minutes: i64) -> Self {
        Self {
            events: RwLock::new(VecDeque::new()),
            max_age: Duration::minutes(max_age_minutes),
        }
    }

    pub fn add(&self, event: String) {
        let now = Utc::now();
        let mut events = self.events.write().unwrap();
        events.push_back((now, event));

        // Prune old events while we have the write lock
        let cutoff = now - self.max_age;
        while let Some((ts, _)) = events.front() {
            if *ts < cutoff {
                events.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn get_all(&self) -> Vec<String> {
        let events = self.events.read().unwrap();
        let now = Utc::now();
        let cutoff = now - self.max_age;

        events
            .iter()
            .filter(|(ts, _)| *ts >= cutoff)
            .map(|(_, msg)| msg.clone())
            .collect()
    }
}
