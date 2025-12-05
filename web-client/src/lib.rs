use wasm_bindgen::prelude::*;
use log::info;

#[wasm_bindgen(start)]
fn start() {
    wasm_logger::init(wasm_logger::Config::default());
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn foo(){
    info!("HI");
}