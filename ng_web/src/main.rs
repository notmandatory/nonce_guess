mod app;
mod block_entry;
mod field_entry;
mod guess_entry;

use crate::app::App;

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
