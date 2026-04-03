pub mod extractors;

use wasm_bindgen::prelude::*;
use serde::Serialize;

#[derive(Serialize)]
pub struct ExtractionResult {
    pub texts: Vec<String>,
    pub success: bool,
    pub error: Option<String>,
    pub pages: u32,
}

#[wasm_bindgen]
pub fn extract(data: &[u8], extension: &str) -> JsValue {
    let result = match extension {
        "pdf" => extractors::pdf::extract(data),
        "docx" => extractors::docx::extract(data),
        "xlsx" => extractors::xlsx::extract(data),
        "pptx" => extractors::pptx::extract(data),
        "txt" => {
            let text = String::from_utf8_lossy(data).to_string();
            ExtractionResult {
                texts: vec![text],
                success: true,
                error: None,
                pages: 1,
            }
        },
        _ => ExtractionResult {
            texts: vec![],
            success: false,
            error: Some(format!("Unsupported format: {}", extension)),
            pages: 0,
        },
    };

    serde_wasm_bindgen::to_value(&result).unwrap()
}
