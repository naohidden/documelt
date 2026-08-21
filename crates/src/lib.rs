pub mod block;
pub mod extractors;

use block::{blocks_to_text, Block, Run};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
pub struct ExtractionResult {
    /// ページ(スライド/シート)単位のプレーンテキスト
    pub texts: Vec<String>,
    /// ページ単位の構造化ブロック。TS 側が Markdown へ整形する
    pub blocks: Vec<Vec<Block>>,
    pub success: bool,
    pub error: Option<String>,
    pub pages: u32,
}

impl ExtractionResult {
    pub fn failure(error: String) -> Self {
        ExtractionResult {
            texts: vec![],
            blocks: vec![],
            success: false,
            error: Some(error),
            pages: 0,
        }
    }

    /// 構造化ブロックから生成する (texts は blocks から復元)
    pub fn from_pages(pages: Vec<Vec<Block>>) -> Self {
        let texts: Vec<String> = pages.iter().map(|b| blocks_to_text(b)).collect();
        let count = pages.len() as u32;
        let success = texts.iter().any(|t| !t.is_empty());
        ExtractionResult {
            texts,
            blocks: pages,
            success,
            error: None,
            pages: count,
        }
    }

    /// 構造を持たない抽出結果 (プレーンテキストのみ) から生成する
    pub fn from_texts(texts: Vec<String>) -> Self {
        let blocks: Vec<Vec<Block>> = texts
            .iter()
            .map(|t| {
                if t.is_empty() {
                    vec![]
                } else {
                    vec![Block::Para {
                        runs: vec![Run::text(t.clone())],
                    }]
                }
            })
            .collect();
        let count = texts.len() as u32;
        let success = texts.iter().any(|t| !t.is_empty());
        ExtractionResult {
            texts,
            blocks,
            success,
            error: None,
            pages: count,
        }
    }
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
            ExtractionResult::from_texts(vec![text])
        }
        _ => ExtractionResult::failure(format!("Unsupported format: {}", extension)),
    };

    serde_wasm_bindgen::to_value(&result).unwrap()
}
