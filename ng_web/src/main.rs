use yew::Renderer;

mod app;
mod block_entry;
mod field_entry;
mod guess_entry;

use crate::app::Main;

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    Renderer::<Main>::new().render();
}
