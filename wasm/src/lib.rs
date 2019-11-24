use bluepaper_core::MarkdownToLatex;

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

lazy_static! {
    #[allow(non_upper_case_globals)]
    static ref images: Mutex<HashMap<String, (String, Vec<u8>)>> = Mutex::new(HashMap::new());
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
    images.lock().unwrap().insert(url, (filename, data));
}

#[wasm_bindgen]
pub fn clear_registered_images() {
    images.lock().unwrap().clear();
}

#[wasm_bindgen]
pub fn latex_to_zipped_latex(latex: String) -> Vec<u8> {
    let zip_file = Vec::new();
    let mut zip_writer = zip::ZipWriter::new(Cursor::new(zip_file));

    let images_guard = images.lock().unwrap();

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

    let images_guard = images.lock().unwrap();

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
