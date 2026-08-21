use documelt::block::{Block, Run};
use documelt::extractors::{docx, pdf, pptx, xlsx};

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
    assert!(joined.contains("documelt"));
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
fn test_xlsx_sheet_name_is_heading_block() {
    let data = read_sample("sample.xlsx");
    let result = xlsx::extract(&data);
    // シート名は Heading ブロックとして持つ。プレーンテキストに Markdown 記法は混ぜない
    assert!(!result.texts[0].contains("## "));
    assert!(matches!(
        result.blocks[0].first(),
        Some(Block::Heading { level: 2, .. })
    ));
}

#[test]
fn test_xlsx_body_is_table_block() {
    let data = read_sample("sample.xlsx");
    let result = xlsx::extract(&data);
    assert!(result.blocks[0]
        .iter()
        .any(|b| matches!(b, Block::Table { .. })));
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
    assert!(text.contains("documelt"));
}


// ========== 構造化ブロック (sample_rich.docx) ==========

fn rich_blocks() -> Vec<Block> {
    let data = read_sample("sample_rich.docx");
    let result = docx::extract(&data);
    assert!(result.success);
    result.blocks.into_iter().next().unwrap()
}

fn find_runs(blocks: &[Block]) -> Vec<Run> {
    let mut out = Vec::new();
    for b in blocks {
        match b {
            Block::Heading { runs, .. } | Block::Para { runs } => out.extend(runs.clone()),
            Block::List { items, .. } => {
                for i in items {
                    out.extend(i.runs.clone());
                }
            }
            Block::Table { rows } => {
                for row in rows {
                    for cell in row {
                        out.extend(cell.clone());
                    }
                }
            }
            Block::Code { .. } => {}
        }
    }
    out
}

#[test]
fn test_docx_headings() {
    let blocks = rich_blocks();
    assert!(matches!(blocks.first(), Some(Block::Heading { level: 1, .. })));
    assert!(blocks
        .iter()
        .any(|b| matches!(b, Block::Heading { level: 2, .. })));
}

#[test]
fn test_docx_bold_italic_strike_highlight() {
    let runs = find_runs(&rich_blocks());
    assert!(runs.iter().any(|r| r.bold && r.text.contains("太字")));
    assert!(runs.iter().any(|r| r.italic && r.text.contains("斜体")));
    assert!(runs.iter().any(|r| r.strike && r.text.contains("取り消し線")));
    assert!(runs
        .iter()
        .any(|r| r.highlight.as_deref() == Some("yellow") && r.text.contains("ハイライト")));
}

#[test]
fn test_docx_hyperlink() {
    let runs = find_runs(&rich_blocks());
    let link = runs
        .iter()
        .find(|r| r.text.contains("リンク"))
        .expect("リンクの Run が見つからない");
    assert_eq!(
        link.link.as_deref(),
        Some("https://github.com/naohidden/documelt")
    );
}

#[test]
fn test_docx_table_structure() {
    let blocks = rich_blocks();
    let table = blocks
        .iter()
        .find_map(|b| match b {
            Block::Table { rows } => Some(rows),
            _ => None,
        })
        .expect("Table ブロックが見つからない");
    assert_eq!(table.len(), 3);
    assert_eq!(table[0].len(), 3);
    assert_eq!(table[0][0][0].text, "Format");
    assert!(table[0][0][0].bold, "ヘッダ行は太字のはず");
    assert_eq!(table[1][1][0].text, "2.3MB");
}

#[test]
fn test_docx_lists_ordered_and_unordered() {
    let blocks = rich_blocks();
    let lists: Vec<(bool, usize)> = blocks
        .iter()
        .filter_map(|b| match b {
            Block::List { ordered, items } => Some((*ordered, items.len())),
            _ => None,
        })
        .collect();
    assert_eq!(lists.len(), 2, "箇条書きと番号リストで2ブロックのはず");
    assert_eq!(lists[0], (false, 2));
    assert_eq!(lists[1], (true, 2));
}

#[test]
fn test_docx_plain_text_has_no_markdown_syntax() {
    let data = read_sample("sample_rich.docx");
    let result = docx::extract(&data);
    let text = &result.texts[0];
    assert!(text.contains("太字"));
    assert!(!text.contains("**"), "プレーンテキストに Markdown 記法を混ぜない");
    assert!(!text.contains("# "));
}
