use std::collections::HashMap;

use wasm_bindgen::prelude::*;
use web_sys::HtmlInputElement;
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
        <ColorGrid/>
    }
}

enum Msg {
    SplitRow { x: usize, y: usize },
    SplitColumn { x: usize, y: usize },
    Merge { x: usize, y: usize, w: usize, h: usize },
    SetColor { x: usize, y: usize, color: String },
    SetContextMenuState(bool),
}
#[derive(Properties, PartialEq)]
struct ColorGrid {
    width: usize,
    height: usize,
    colors: HashMap<(usize, usize), (usize, usize, String)>,
    contextmenu_hidden: bool,
}

fn default_color() -> String {
    String::from("#000000")
}

impl ColorGrid {
    fn main(&self, ctx: &Context<Self>) -> Html {
        let colors = &self.colors;
        let width = self.width;
        let height = self.height;
        let oninput = ctx.link().batch_callback(|e: InputEvent| {
            e.target_dyn_into::<HtmlInputElement>().and_then(|e| {
                e.parent_element().and_then(|p| {
                    p.get_attribute("data-position")
                        .and_then(|attr| attr.split_once(' ').map(|(y, x)| (y.parse().unwrap(), x.parse().unwrap())))
                        .map(|(y, x): (usize, usize)| Msg::SetColor { x, y, color: e.value() })
                })
            })
        });

        html! {
            <div style={format!("\
                display: grid; gap: 0;\
                grid-template-rows: repeat({}, minmax(0, 1fr));\
                grid-template-columns: repeat({}, minmax(0, 1fr));
                place-items: center; overflow:hidden; ", height, width)}>
                {
                    colors.iter().map(|((y, x), (h, w, color))| {
                        let area_rule = format!("grid-area: {y} / {x} / span {h} / span {w}");
                        let container_style = format!("\
                            {area_rule};\
                            display: grid;\
                            grid-template: main 1fr / 1fr;\
                            background-color: {color};");
                        let input_style =format!("max-width: 11em; max-height: 5em; grid-area: main;");
                        let hex_display_style = format!("\
                            max-width: min-content; max-height: 2em;\
                            min-width: 0; min-height: 0;\
                            color:white;\
                            filter: drop-shadow(0 0 6px black);\
                            text-overflow: ellipsis;\
                            overflow: hidden;
                            grid-area: main;");
                        html!{
                            <div key={format!("{y}/{x}")} style={container_style} data-position={format!("{} {}", y, x)}>
                                <input type="color"
                                       oninput={oninput.clone()}
                                       value={color.clone()}
                                       style={input_style}
                                       draggable="true"/>
                                <p style={hex_display_style} >{color.clone()}</p>
                            </div>
                        }
                    }).collect::<Html>()
                }
            </div>
        }
    }
    fn contextmenu(&self, ctx: &Context<Self>) -> Html {
        if self.contextmenu_hidden {
            return html!(<></>);
        }
        html! {
            <></>
        }
    }
}

impl Component for ColorGrid {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            width: 1,
            height: 1,
            colors: {
                let mut hash = HashMap::new();

                hash.insert((0, 0), (1, 1, default_color()));
                hash
            },
            contextmenu_hidden: true,
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
            {self.main(ctx)}
            {self.contextmenu(ctx)}
            </>
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::SetColor { x, y, color } => {
                self.colors.get_mut(&(x, y)).iter_mut().for_each(|c| **c = (1, 1, color.clone()));
                true
            }
            Msg::SplitRow { x, y } => todo!(),
            Msg::SplitColumn { x, y } => todo!(),
            Msg::Merge { x, y, w, h } => todo!(),
            Msg::SetContextMenuState(active) => {
                self.contextmenu_hidden = !active;
                true
            }
        }
    }
}
