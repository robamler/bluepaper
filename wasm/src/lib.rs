use bluepaper_core::MarkdownToLatex;

use wasm_bindgen::prelude::*;
use web_sys::console;
use zip::write::FileOptions;

use std::io::Cursor;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn markdown_to_latex(markdown: Vec<u8>) -> String {
    let markdown = String::from_utf8(markdown).unwrap();
    MarkdownToLatex::from_string(markdown).into_string()
}

#[wasm_bindgen]
pub fn markdown_to_zipped_latex(markdown: Vec<u8>, file_name: String) -> Vec<u8> {
    let zip_file = Vec::new();
    let mut zip_writer = zip::ZipWriter::new(Cursor::new(zip_file));

    zip_writer
        .add_directory("tex/", Default::default())
        .unwrap();
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .unix_permissions(0o755);
    zip_writer
        .start_file(format!("tex/{}", file_name), options)
        .unwrap();

    MarkdownToLatex::from_string(String::from_utf8(markdown).unwrap())
        .write_to(&mut zip_writer)
        .unwrap();

    zip_writer.finish().unwrap().into_inner()
}
