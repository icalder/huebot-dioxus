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
        div {
            id: "navbar",
            class: "flex flex-row mb-5",
            Link {
                class: "text-white mr-5 no-underline transition-colors duration-200 hover:cursor-pointer hover:text-[#91a4d2]",
                to: Route::Home {},
                "Home"
            }
            Link {
                class: "text-white mr-5 no-underline transition-colors duration-200 hover:cursor-pointer hover:text-[#91a4d2]",
                to: Route::Blog { id: 1 },
                "Blog"
            }
            Link {
                class: "text-white mr-5 no-underline transition-colors duration-200 hover:cursor-pointer hover:text-[#91a4d2]",
                to: Route::Sensors {},
                "Sensors"
            }
        }

        // The `Outlet` component is used to render the next component inside the layout. In this case, it will render either
        // the [`Home`] or [`Blog`] component depending on the current route.
        Outlet::<Route> {}
    }
}
