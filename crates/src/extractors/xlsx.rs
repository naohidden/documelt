use crate::ExtractionResult;
use calamine::{Reader, Xlsx};
use std::io::Cursor;

pub fn extract(data: &[u8]) -> ExtractionResult {
    let cursor = Cursor::new(data);
    let mut workbook: Xlsx<_> = match Xlsx::new(cursor) {
        Ok(wb) => wb,
        Err(e) => {
            return ExtractionResult {
                texts: vec![],
                success: false,
                error: Some(format!("Failed to open XLSX: {}", e)),
                pages: 0,
            }
        }
    };

    let sheet_names: Vec<String> = workbook.sheet_names().to_vec();
    let mut texts: Vec<String> = Vec::new();

    for name in &sheet_names {
        if let Ok(range) = workbook.worksheet_range(name) {
            let mut sheet_text = format!("## {}\n", name);
            for row in range.rows() {
                let cells: Vec<String> = row
                    .iter()
                    .map(|cell| format!("{}", cell))
                    .collect();
                sheet_text.push_str(&cells.join("\t"));
                sheet_text.push('\n');
            }
            texts.push(sheet_text.trim().to_string());
        }
    }

    let success = texts.iter().any(|t| !t.is_empty());
    let pages = texts.len() as u32;
    ExtractionResult {
        texts: if success { texts } else { vec![] },
        success,
        error: None,
        pages,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_data_returns_error() {
        let result = extract(b"not an xlsx file");
        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("Failed to open XLSX"));
    }

    #[test]
    fn test_empty_data_returns_error() {
        let result = extract(b"");
        assert!(!result.success);
    }
}
