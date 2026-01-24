//! The views module contains the components for all Layouts and Routes for our app. Each layout and route in our [`Route`]
//! enum will render one of these components.
//!
//!
//! The [`Home`] and other components will be rendered when the current route is for example [`Route::Home`].
//!
//!
//! The [`Navbar`] component will be rendered on all pages of our app since every page is under the layout. The layout defines
//! a common wrapper around all child routes.

mod home;
pub use home::Home;

mod sensors;
pub use sensors::Sensors;

mod navbar;
pub use navbar::Navbar;

mod event_log;
pub use event_log::EventLog;
