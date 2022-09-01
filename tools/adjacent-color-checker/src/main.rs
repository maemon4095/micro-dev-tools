use wasm_bindgen::prelude::*;
use yew::prelude::*;

fn main() {
    let app = web_sys::window()
        .expect_throw("window was not found.")
        .document()
        .expect_throw("document was not found in window.")
        .get_element_by_id("#app")
        .expect_throw("app was not found in document.");

    yew::start_app_in_element::<App>(app);
}

#[function_component(App)]
fn app() -> Html {
    html! {
        <a>{"yo!"}</a>
    }
}
