use maud::html;

fn page() -> maud::Markup {
    html! {
        div class="mb-8" { "hello" }
    }
}
