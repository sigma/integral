//! WASM bindings for the OpenLink Integra-7 control surface.

use wasm_bindgen::prelude::*;

/// Placeholder: returns the library version.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
