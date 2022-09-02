use wasm_bindgen::prelude::*;
use yew::prelude::*;

#[function_component(Home)]
pub fn home() -> Html {
    html! {
        <a href="tools/adjacent-color-checker"> { "home!" } </a>
    }
}
