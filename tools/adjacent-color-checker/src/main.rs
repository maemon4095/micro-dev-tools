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
        <ColorGrid width={2} height={2}/>
    }
}

enum Msg {
    RemoveRow { from: f32, to: f32 },
    RemoveColumn { from: f32, to: f32 },
    AddRow { index: isize },
    AddColumn { index: isize },
    SetColor { x: usize, y: usize, color: String },
}
#[derive(Properties, PartialEq)]
struct Props {
    width: usize,
    height: usize,
}
struct ColorGrid {
    width: usize,
    height: usize,
    colors: Vec<Vec<String>>,
}

fn default_color() -> String {
    String::from("#000000")
}
impl Component for ColorGrid {
    type Message = Msg;

    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let props = ctx.props();
        Self {
            width: props.width,
            height: props.height,
            colors: vec![vec![default_color(); props.width]; props.height],
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let colors = &self.colors;
        let width = self.width;
        let height = self.height;
        let onclick_up = ctx.link().callback(|_| Msg::AddRow { index: 0 });
        let onclick_down = ctx.link().callback(|_| Msg::AddRow { index: -1 });
        let onclick_left = ctx.link().callback(|_| Msg::AddColumn { index: 0 });
        let onclick_right = ctx.link().callback(|_| Msg::AddColumn { index: -1 });
        let oninput = ctx.link().batch_callback(|e: InputEvent| {
            e.target_dyn_into::<HtmlInputElement>().and_then(|e| {
                e.get_attribute("data-position")
                    .and_then(|attr| attr.split_once(' ').map(|(y, x)| (y.parse().unwrap(), x.parse().unwrap())))
                    .map(|(y, x): (usize, usize)| Msg::SetColor { x, y, color: e.value() })
            })
        });
        html! {
            <div style={format!("display: grid; gap: 0; grid-template-rows: 15px repeat({}, 1fr) 15px; grid-template-columns: 15px repeat({}, 1fr) 15px; place-items: center; overflow:hidden;", height, width)}>
                <button style="grid-row: 1; grid-column: 2 / -2"
                        draggable="true"
                        onclick={onclick_up}>//up
                </button>
                <button style="grid-row: -2; grid-column: 2 / -2"
                        draggable="true"
                        onclick={onclick_down}>//down
                </button>
                <button style="grid-row: 2 / -2; grid-column: 1"
                        draggable="true"
                        onclick={onclick_left}>//left
                </button>
                <button style="grid-row: 2 / -2; grid-column: -2"
                        draggable="true"
                        onclick={onclick_right}>//right
                </button>
                {
                    colors.iter().enumerate().flat_map(|(y, row)| row.iter().enumerate().map(move |(x, e)| (x, y, e))).map(|(x, y, e)| {
                        let area_rule = format!("grid-area: {}/{}", y + 2, x + 2);
                        let color_display_style = format!("{area_rule}; pointer-events: none; background-color: {e};");
                        let input_style =format!("{area_rule}; width: 33%; height: 33%;");
                        let hex_display_style = format!("{area_rule}; width: max-content; height: max-content; color:white; filter: drop-shadow(0 0 6px black);");
                        html!{
                            <>
                                <input data-position={format!("{} {}", y, x)} oninput={oninput.clone()} type="color" value={e.clone()} style={input_style}/>
                                <div style={color_display_style}/>
                                <p style={hex_display_style} >{e.clone()}</p>
                            </>
                        }
                    }).collect::<Html>()
                }
            </div>
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::RemoveRow { from, to } => todo!(),
            Msg::RemoveColumn { from, to } => todo!(),
            Msg::AddRow { index } => {
                let index = if index < 0 { 1 + index + self.height as isize } else { index } as usize;
                self.colors.insert(index, vec![default_color(); self.width]);
                self.height += 1;
                true
            }
            Msg::AddColumn { index } => {
                let index = if index < 0 { 1 + index + self.width as isize } else { index } as usize;
                for row in self.colors.iter_mut() {
                    row.insert(index, default_color())
                }
                self.width += 1;
                true
            }
            Msg::SetColor { x, y, color } => {
                self.colors[y][x] = color;
                true
            }
        }
    }
}
