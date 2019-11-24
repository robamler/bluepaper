use bluepaper_core::{format::WhitespaceFormatter, latex_escape::escape_str, MarkdownToLatex};

use js_sys;
use lazy_static::lazy_static;
use std::io::Write;
use wasm_bindgen::prelude::*;
use zip::write::FileOptions;

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Mutex;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

mod globals {
    #![allow(non_upper_case_globals)]
    use super::{lazy_static, HashMap, Mutex, WhitespaceFormatter};

    lazy_static! {
        pub static ref images: Mutex<HashMap<String, (String, Vec<u8>)>> =
            Mutex::new(HashMap::new());
        pub static ref latex_formatter: Mutex<WhitespaceFormatter<Vec<u8>>> =
            Mutex::new(WhitespaceFormatter::new_latex_formatter(Vec::new()).unwrap());
    }
}

#[wasm_bindgen]
pub fn markdown_to_latex(markdown: String, image_callback: &js_sys::Function) -> String {
    clear_registered_images();

    let this = JsValue::NULL;
    MarkdownToLatex::from_string(markdown).into_string_with_image_callback(&mut |url| {
        image_callback.call1(&this, &JsValue::from(url)).unwrap();
        None
    })
}

#[wasm_bindgen]
pub fn register_image(url: String, filename: String, data: Vec<u8>) {
    globals::images
        .lock()
        .unwrap()
        .insert(url, (filename, data));
}

#[wasm_bindgen]
pub fn clear_registered_images() {
    globals::images.lock().unwrap().clear();
}

#[wasm_bindgen]
pub fn latex_to_zipped_latex(latex: String) -> Vec<u8> {
    let zip_file = Vec::new();
    let mut zip_writer = zip::ZipWriter::new(Cursor::new(zip_file));

    let images_guard = globals::images.lock().unwrap();

    zip_writer
        .add_directory("figures/", Default::default())
        .unwrap();
    let zip_options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .unix_permissions(0o755);

    for (filename, (_, data)) in images_guard.iter() {
        let path = format!("figures/{}", filename);
        zip_writer.start_file(&path, zip_options).unwrap();
        zip_writer.write_all(data).unwrap();
    }

    zip_writer.start_file("main.tex", zip_options).unwrap();
    zip_writer.write_all(latex.as_bytes()).unwrap();

    zip_writer.finish().unwrap().into_inner()
}

#[wasm_bindgen]
pub fn markdown_to_zipped_latex(markdown: String) -> Vec<u8> {
    let zip_file = Vec::new();
    let mut zip_writer = zip::ZipWriter::new(Cursor::new(zip_file));

    let images_guard = globals::images.lock().unwrap();

    zip_writer
        .add_directory("figures/", Default::default())
        .unwrap();
    let zip_options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .unix_permissions(0o755);

    let latex =
        MarkdownToLatex::from_string(markdown).into_string_with_image_callback(&mut |url| {
            if let Some((filename, data)) = images_guard.get(url) {
                let path = format!("figures/{}", filename);
                zip_writer.start_file(&path, zip_options).unwrap();
                zip_writer.write_all(data).unwrap();
                Some(path)
            } else {
                None
            }
        });

    zip_writer.start_file("main.tex", zip_options).unwrap();
    zip_writer.write_all(latex.as_bytes()).unwrap();

    zip_writer.finish().unwrap().into_inner()
}

#[wasm_bindgen]
pub fn clear_output() -> String {
    let mut formatter = globals::latex_formatter.lock().unwrap();
    let output = unsafe { std::str::from_utf8_unchecked(formatter.get_mut().unwrap()) };
    let output = output.to_string();
    formatter.reset_latex_formatter();
    output
}

#[wasm_bindgen]
pub fn write_raw(s: &str, newlines_before: u32, newlines_after: u32) {
    let mut formatter = globals::latex_formatter.lock().unwrap();
    formatter.add_newlines(newlines_before);
    formatter.write_all(s.as_bytes()).unwrap();
    formatter.add_newlines(newlines_after);
}

#[wasm_bindgen]
pub fn write_escaped(s: &str, newlines_before: u32, newlines_after: u32) {
    let mut formatter = globals::latex_formatter.lock().unwrap();
    formatter.add_newlines(newlines_before);
    escape_str(s, formatter.get_mut().unwrap()).unwrap();
    formatter.add_newlines(newlines_after);
}

#[wasm_bindgen]
pub fn add_newlines(num: u32) {
    globals::latex_formatter.lock().unwrap().add_newlines(num);
}

#[wasm_bindgen]
pub fn increase_indent() {
    globals::latex_formatter.lock().unwrap().increase_indent();
}

#[wasm_bindgen]
pub fn decrease_indent() {
    globals::latex_formatter.lock().unwrap().decrease_indent();
}

#[wasm_bindgen]
pub fn limit_newlines(num: u32) {
    globals::latex_formatter.lock().unwrap().limit_newlines(num);
}
