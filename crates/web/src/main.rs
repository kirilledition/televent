use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        div {
            class: "min-h-screen bg-zinc-950 text-zinc-50 flex items-center justify-center",
            div {
                class: "text-center",
                h1 {
                    class: "text-4xl font-bold mb-4",
                    "Televent"
                }
                p {
                    class: "text-zinc-400",
                    "Telegram-native calendar management"
                }
            }
        }
    }
}
