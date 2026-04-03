use documelt::extractors::{pdf, docx, xlsx, pptx};

const SAMPLES_DIR: &str = "../samples";

fn read_sample(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/{}", SAMPLES_DIR, name))
        .unwrap_or_else(|e| panic!("Failed to read sample file {}: {}", name, e))
}

// ========== PDF ==========

#[test]
fn test_pdf_extracts_text() {
    let data = read_sample("sample.pdf");
    let result = pdf::extract(&data);
    assert!(result.success);
    assert!(!result.texts.is_empty());
    let joined = result.texts.join("\n");
    assert!(joined.contains("SHELP"));
}

#[test]
fn test_pdf_returns_pages() {
    let data = read_sample("sample.pdf");
    let result = pdf::extract(&data);
    assert!(result.pages > 0);
    assert_eq!(result.pages as usize, result.texts.len());
}

#[test]
fn test_pdf_outline_fails_gracefully() {
    let data = read_sample("sample-ouline.pdf");
    let result = pdf::extract(&data);
    assert!(!result.success);
    assert!(result.texts.is_empty());
    assert!(result.error.is_some());
}

// ========== DOCX ==========

#[test]
fn test_docx_extracts_text() {
    let data = read_sample("sample.docx");
    let result = docx::extract(&data);
    assert!(result.success);
    assert_eq!(result.texts.len(), 1);
    assert!(result.texts[0].len() > 100);
}

#[test]
fn test_docx_preserves_paragraphs() {
    let data = read_sample("sample.docx");
    let result = docx::extract(&data);
    assert!(result.texts[0].contains('\n'));
}

// ========== XLSX ==========

#[test]
fn test_xlsx_extracts_text() {
    let data = read_sample("sample.xlsx");
    let result = xlsx::extract(&data);
    assert!(result.success);
    assert!(!result.texts.is_empty());
}

#[test]
fn test_xlsx_returns_per_sheet() {
    let data = read_sample("sample.xlsx");
    let result = xlsx::extract(&data);
    assert!(result.pages > 0);
    assert_eq!(result.pages as usize, result.texts.len());
}

#[test]
fn test_xlsx_includes_sheet_name() {
    let data = read_sample("sample.xlsx");
    let result = xlsx::extract(&data);
    assert!(result.texts[0].contains("## "));
}

#[test]
fn test_xlsx_tab_separated() {
    let data = read_sample("sample.xlsx");
    let result = xlsx::extract(&data);
    assert!(result.texts[0].contains('\t'));
}

// ========== PPTX ==========

#[test]
fn test_pptx_extracts_text() {
    let data = read_sample("sample.pptx");
    let result = pptx::extract(&data);
    assert!(result.success);
    assert!(!result.texts.is_empty());
}

#[test]
fn test_pptx_returns_per_slide() {
    let data = read_sample("sample.pptx");
    let result = pptx::extract(&data);
    assert!(result.pages > 0);
    assert_eq!(result.pages as usize, result.texts.len());
}

// ========== TXT ==========

#[test]
fn test_txt_roundtrip() {
    let data = read_sample("sample.txt");
    let text = String::from_utf8_lossy(&data).to_string();
    assert!(text.contains("テスト用"));
}
