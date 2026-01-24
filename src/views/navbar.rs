use crate::Route;
use dioxus::prelude::*;

/// The Navbar component that will be rendered on all pages of our app since every page is under the layout.
///
///
/// This layout component wraps the UI of [Route::Home] and [Route::Blog] in a common navbar. The contents of the Home and Blog
/// routes will be rendered under the outlet inside this component
#[component]
pub fn Navbar() -> Element {
    rsx! {
        div { id: "navbar", class: "flex flex-row mb-5",
            Link { class: "nav-link", to: Route::Home {}, "Home" }
            Link { class: "nav-link", to: Route::Sensors {}, "Sensors" }
        }

        // The `Outlet` component is used to render the next component inside the layout. In this case, it will render either
        // the [`Home`] or [`Blog`] component depending on the current route.
        SuspenseBoundary {
            fallback: move |_| rsx! {
                div {
                    width: "100%",
                    height: "100%",
                    display: "flex",
                    align_items: "center",
                    justify_content: "center",
                    "Loading..."
                }
            },
            Outlet::<Route> {}
        }
    }
}
