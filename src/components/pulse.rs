use dioxus::prelude::*;

#[component]
pub fn Pulsing(
    /// A value that, when changed, triggers the pulse animation.
    trigger: String,
    children: Element,
    #[props(into, default = "animate-pulse-once".to_string())] animation: String,
    #[props(into, default = String::new())] class: String,
) -> Element {
    rsx! {
        div {
            class: "{class} {animation}",
            {children}
        }
    }
}