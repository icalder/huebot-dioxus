use dioxus::prelude::*;

const HEADER_SVG: Asset = asset!("/assets/header.svg");

#[component]
pub fn Hero() -> Element {
    rsx! {
        // We can create elements inside the rsx macro with the element name followed by a block of attributes and children.
        div {
            // Attributes should be defined in the element before any children
            id: "hero",
            class: "flex flex-col justify-center items-center",
            // After all attributes are defined, we can define child elements and components
            img {
                src: HEADER_SVG,
                id: "header",
                class: "max-w-[1200px]"
            }
            div {
                id: "links",
                class: "w-[400px] text-left text-xl text-white flex flex-col",
                // The RSX macro also supports text nodes surrounded by quotes
                a {
                    class: "text-white no-underline my-2.5 border border-white rounded p-2.5 hover:bg-[#1f1f1f] hover:cursor-pointer",
                    href: "https://dioxuslabs.com/learn/0.7/",
                    "📚 Learn Dioxus"
                }
                a {
                    class: "text-white no-underline my-2.5 border border-white rounded p-2.5 hover:bg-[#1f1f1f] hover:cursor-pointer",
                    href: "https://dioxuslabs.com/awesome",
                    "🚀 Awesome Dioxus"
                }
                a {
                    class: "text-white no-underline my-2.5 border border-white rounded p-2.5 hover:bg-[#1f1f1f] hover:cursor-pointer",
                    href: "https://github.com/dioxus-community/",
                    "📡 Community Libraries"
                }
                a {
                    class: "text-white no-underline my-2.5 border border-white rounded p-2.5 hover:bg-[#1f1f1f] hover:cursor-pointer",
                    href: "https://github.com/DioxusLabs/sdk",
                    "⚙️ Dioxus Development Kit"
                }
                a {
                    class: "text-white no-underline my-2.5 border border-white rounded p-2.5 hover:bg-[#1f1f1f] hover:cursor-pointer",
                    href: "https://marketplace.visualstudio.com/items?itemName=DioxusLabs.dioxus",
                    "💫 VSCode Extension"
                }
                a {
                    class: "text-white no-underline my-2.5 border border-white rounded p-2.5 hover:bg-[#1f1f1f] hover:cursor-pointer",
                    href: "https://discord.gg/XgGxMSkvUM",
                    "👋 Community Discord"
                }
            }
        }
    }
}
