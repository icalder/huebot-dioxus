//! The components module contains all shared components for our app. Components are the building blocks of dioxus apps.
//! They can be used to defined common UI elements like buttons, forms, and modals.

mod sensor;
pub use sensor::Sensor;

mod clock;
pub use clock::Clock;

mod sparkline;
pub use sparkline::{Sparkline, HistoryPoint};
