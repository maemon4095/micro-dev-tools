mod pages;
use wasm_bindgen::prelude::*;
use yew::prelude::*;
use yew_router::prelude::*;

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
        <BrowserRouter>
            <Switch<Route> render={Switch::render(switch)} />
        </BrowserRouter>
    }
}

fn switch(route: &Route) -> Html {
    match route {
        Route::NotFound => html! { <a> { "not found" } </a>},
        Route::Home => html! { <pages::Home/> },
    }
}

#[derive(Routable, PartialEq, Eq, Clone, Copy)]
enum Route {
    #[not_found]
    #[at("/404")]
    NotFound,
    #[at("/")]
    Home,
}
