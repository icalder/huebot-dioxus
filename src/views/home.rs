use dioxus::prelude::*;

/// The Home page component that will be rendered when the current route is `[Route::Home]`
#[component]
pub fn Home() -> Element {
    rsx! {
        div {
            class: "container mx-auto p-4 text-center",
            h1 {
                class: "text-4xl font-bold mb-4",
                "Huebot Dashboard"
            }
            p {
                class: "text-lg text-gray-600 dark:text-gray-400",
                "Welcome to your Philips Hue sensor monitor."
            }
        }
    }
}
