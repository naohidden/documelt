use crate::ExtractionResult;

/// PDF はテキストのみ抽出する。
/// pdf-extract の by-pages API は文字列しか返さないため、
/// 表・太字・リンクなどの構造情報は復元できない (README の Limitations 参照)。
pub fn extract(data: &[u8]) -> ExtractionResult {
    match pdf_extract::extract_text_from_mem_by_pages(data) {
        Ok(page_texts) => {
            let texts: Vec<String> = page_texts
                .into_iter()
                .map(|t| t.trim().to_string())
                .collect();
            let total_len: usize = texts.iter().map(|t| t.len()).sum();

            if total_len < 10 {
                let mut result = ExtractionResult::failure(
                    "PDF text too short (likely scanned/image PDF)".to_string(),
                );
                result.pages = texts.len() as u32;
                result
            } else {
                ExtractionResult::from_texts(texts)
            }
        }
        Err(e) => ExtractionResult::failure(format!("PDF extraction failed: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_data_returns_error() {
        let result = extract(b"this is not a pdf");
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_empty_data_returns_error() {
        let result = extract(b"");
        assert!(!result.success);
    }

    #[test]
    fn test_short_text_detected_as_scan() {
        let result = extract(b"%PDF-1.4 minimal");
        assert!(!result.success);
    }
}
