use maud::html;

fn page() -> maud::Markup {
    html! {
        div style="margin-bottom: 2rem;" { "hello" }
    }
}
