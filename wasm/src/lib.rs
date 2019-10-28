use bluepaper_core::MarkdownToLatex;

use wasm_bindgen::prelude::*;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn markdown_to_latex(markdown: String) -> String {
    MarkdownToLatex::from_string(markdown).into_string()
}
