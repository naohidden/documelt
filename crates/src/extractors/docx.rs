use crate::block::{merge_runs, Block, ListItem, Run};
use crate::ExtractionResult;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::io::Cursor;
use zip::ZipArchive;

pub fn extract(data: &[u8]) -> ExtractionResult {
    let cursor = Cursor::new(data);
    let mut archive = match ZipArchive::new(cursor) {
        Ok(a) => a,
        Err(e) => return ExtractionResult::failure(format!("Failed to open DOCX: {}", e)),
    };

    let doc = match read_entry(&mut archive, "word/document.xml") {
        Some(d) => d,
        None => return ExtractionResult::failure("word/document.xml not found".to_string()),
    };

    let links = read_rels(&mut archive, "word/_rels/document.xml.rels");
    let numbering = read_numbering(&mut archive);

    let blocks = parse_document(&doc, &links, &numbering);
    if blocks.is_empty() {
        return ExtractionResult::failure("No text found in DOCX".to_string());
    }
    ExtractionResult::from_pages(vec![blocks])
}

fn read_entry(archive: &mut ZipArchive<Cursor<&[u8]>>, name: &str) -> Option<Vec<u8>> {
    let mut file = archive.by_name(name).ok()?;
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut buf).ok()?;
    Some(buf)
}

/// `_rels/*.rels` から rId -> Target を読む (ハイパーリンク解決用)
fn read_rels(archive: &mut ZipArchive<Cursor<&[u8]>>, name: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Some(data) = read_entry(archive, name) else {
        return map;
    };
    let mut reader = Reader::from_reader(data.as_slice());
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) => {
                if e.local_name().as_ref() == b"Relationship" {
                    let mut id = String::new();
                    let mut target = String::new();
                    let mut is_link = false;
                    for a in e.attributes().flatten() {
                        let v = String::from_utf8_lossy(&a.value).to_string();
                        match a.key.local_name().as_ref() {
                            b"Id" => id = v,
                            b"Target" => target = v,
                            b"Type" => is_link = v.ends_with("/hyperlink"),
                            _ => {}
                        }
                    }
                    if is_link && !id.is_empty() {
                        map.insert(id, target);
                    }
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    map
}

/// numbering.xml から numId -> (ilvl -> 順序付きか) を読む
fn read_numbering(archive: &mut ZipArchive<Cursor<&[u8]>>) -> HashMap<String, bool> {
    let mut result = HashMap::new();
    let Some(data) = read_entry(archive, "word/numbering.xml") else {
        return result;
    };

    // abstractNumId -> 最初のレベルが順序付きか
    let mut abstract_ordered: HashMap<String, bool> = HashMap::new();
    let mut num_to_abstract: HashMap<String, String> = HashMap::new();

    let mut reader = Reader::from_reader(data.as_slice());
    let mut buf = Vec::new();
    let mut cur_abstract: Option<String> = None;
    let mut cur_num: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let local = e.local_name().as_ref().to_vec();
                let attr = |n: &[u8]| -> Option<String> {
                    e.attributes().flatten().find_map(|a| {
                        (a.key.local_name().as_ref() == n)
                            .then(|| String::from_utf8_lossy(&a.value).to_string())
                    })
                };
                match local.as_slice() {
                    b"abstractNum" => cur_abstract = attr(b"abstractNumId"),
                    b"num" => cur_num = attr(b"numId"),
                    b"abstractNumId" => {
                        if let (Some(n), Some(v)) = (cur_num.clone(), attr(b"val")) {
                            num_to_abstract.insert(n, v);
                        }
                    }
                    b"numFmt" => {
                        if let (Some(a), Some(v)) = (cur_abstract.clone(), attr(b"val")) {
                            abstract_ordered.entry(a).or_insert(v != "bullet" && v != "none");
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    for (num, abs) in num_to_abstract {
        let ordered = abstract_ordered.get(&abs).copied().unwrap_or(false);
        result.insert(num, ordered);
    }
    result
}

#[derive(Default, Clone)]
struct RunProps {
    bold: bool,
    italic: bool,
    strike: bool,
    code: bool,
    highlight: Option<String>,
}

#[derive(Default)]
struct Para {
    runs: Vec<Run>,
    style: Option<String>,
    num_id: Option<String>,
    ilvl: u8,
}

#[derive(Default)]
struct TableBuilder {
    rows: Vec<Vec<Vec<Run>>>,
    row: Vec<Vec<Run>>,
    cell: Vec<Run>,
}

fn parse_document(
    data: &[u8],
    links: &HashMap<String, String>,
    numbering: &HashMap<String, bool>,
) -> Vec<Block> {
    let mut reader = Reader::from_reader(data);
    let mut buf = Vec::new();

    let mut blocks: Vec<Block> = Vec::new();
    let mut tables: Vec<TableBuilder> = Vec::new();
    let mut para = Para::default();
    let mut rpr = RunProps::default();
    let mut link: Option<String> = None;
    let mut in_text = false;
    let mut in_rpr = false;
    // 連続する箇条書き段落をまとめるバッファ
    let mut list_buf: Vec<ListItem> = Vec::new();
    let mut list_ordered = false;

    loop {
        let ev = reader.read_event_into(&mut buf);
        match ev {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let empty = matches!(ev, Ok(Event::Empty(_)));
                let local = e.local_name().as_ref().to_vec();
                let attr = |n: &[u8]| -> Option<String> {
                    e.attributes().flatten().find_map(|a| {
                        (a.key.local_name().as_ref() == n)
                            .then(|| String::from_utf8_lossy(&a.value).to_string())
                    })
                };
                // w:b / w:i などは w:val="0" で打ち消しになる
                let on = || attr(b"val").map_or(true, |v| v != "0" && v != "false");

                match local.as_slice() {
                    b"p" => para = Para::default(),
                    b"pStyle" => para.style = attr(b"val"),
                    b"numId" => para.num_id = attr(b"val"),
                    b"ilvl" => {
                        para.ilvl = attr(b"val").and_then(|v| v.parse().ok()).unwrap_or(0)
                    }
                    b"rPr" => {
                        in_rpr = true;
                        if empty {
                            in_rpr = false;
                        } else {
                            rpr = RunProps::default();
                        }
                    }
                    b"b" if in_rpr => rpr.bold = on(),
                    b"i" if in_rpr => rpr.italic = on(),
                    b"strike" if in_rpr => rpr.strike = on(),
                    b"highlight" if in_rpr => {
                        rpr.highlight = attr(b"val").filter(|v| v != "none")
                    }
                    b"rStyle" if in_rpr => {
                        if let Some(v) = attr(b"val") {
                            let v = v.to_ascii_lowercase();
                            rpr.code = v.contains("code") || v.contains("verbatim");
                        }
                    }
                    b"hyperlink" => {
                        link = attr(b"id").and_then(|id| links.get(&id).cloned());
                    }
                    b"t" => in_text = true,
                    b"br" => para.runs.push(Run::text("\n")),
                    b"tab" => para.runs.push(Run::text("\t")),
                    b"tbl" => tables.push(TableBuilder::default()),
                    b"tr" => {
                        if let Some(t) = tables.last_mut() {
                            t.row = Vec::new();
                        }
                    }
                    b"tc" => {
                        if let Some(t) = tables.last_mut() {
                            t.cell = Vec::new();
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_text {
                    if let Ok(t) = e.unescape() {
                        para.runs.push(Run {
                            text: t.to_string(),
                            bold: rpr.bold,
                            italic: rpr.italic,
                            strike: rpr.strike,
                            code: rpr.code,
                            highlight: rpr.highlight.clone(),
                            link: link.clone(),
                        });
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                match e.local_name().as_ref() {
                    b"t" => in_text = false,
                    b"rPr" => in_rpr = false,
                    b"r" => rpr = RunProps::default(),
                    b"hyperlink" => link = None,
                    b"p" => {
                        let runs = merge_runs(std::mem::take(&mut para.runs));
                        // 表のセル内では段落を連結してセルに積む
                        if let Some(t) = tables.last_mut() {
                            if !runs.is_empty() {
                                if !t.cell.is_empty() {
                                    t.cell.push(Run::text(" "));
                                }
                                t.cell.extend(runs);
                            }
                        } else if let Some(item) = as_list_item(&para, &runs, numbering) {
                            let ordered = numbering
                                .get(para.num_id.as_deref().unwrap_or(""))
                                .copied()
                                .unwrap_or(false);
                            if !list_buf.is_empty() && ordered != list_ordered {
                                flush_list(&mut blocks, &mut list_buf, list_ordered);
                            }
                            list_ordered = ordered;
                            list_buf.push(item);
                        } else {
                            flush_list(&mut blocks, &mut list_buf, list_ordered);
                            if runs.is_empty() {
                                // 空段落は捨てる
                            } else if let Some(level) = heading_level(para.style.as_deref()) {
                                blocks.push(Block::Heading { level, runs });
                            } else {
                                blocks.push(Block::Para { runs });
                            }
                        }
                    }
                    b"tc" => {
                        if let Some(t) = tables.last_mut() {
                            let cell = merge_runs(std::mem::take(&mut t.cell));
                            t.row.push(cell);
                        }
                    }
                    b"tr" => {
                        if let Some(t) = tables.last_mut() {
                            let row = std::mem::take(&mut t.row);
                            if !row.is_empty() {
                                t.rows.push(row);
                            }
                        }
                    }
                    b"tbl" => {
                        if let Some(t) = tables.pop() {
                            if !t.rows.is_empty() {
                                let table = Block::Table { rows: t.rows };
                                // ネストした表は親セルに入れられないので親ブロックへ
                                flush_list(&mut blocks, &mut list_buf, list_ordered);
                                blocks.push(table);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    flush_list(&mut blocks, &mut list_buf, list_ordered);
    blocks
}

fn as_list_item(para: &Para, runs: &[Run], numbering: &HashMap<String, bool>) -> Option<ListItem> {
    if runs.is_empty() {
        return None;
    }
    let num_id = para.num_id.as_deref()?;
    // numbering.xml に無い numId でも、numPr が付いていれば箇条書き扱い
    let _ = numbering;
    if num_id == "0" {
        return None;
    }
    Some(ListItem {
        level: para.ilvl,
        runs: runs.to_vec(),
    })
}

fn flush_list(blocks: &mut Vec<Block>, buf: &mut Vec<ListItem>, ordered: bool) {
    if buf.is_empty() {
        return;
    }
    blocks.push(Block::List {
        ordered,
        items: std::mem::take(buf),
    });
}

/// `Heading1` / `Heading 2` / `Title` などから見出しレベルを判定
fn heading_level(style: Option<&str>) -> Option<u8> {
    let s = style?;
    let lower = s.to_ascii_lowercase().replace([' ', '-', '_'], "");
    if lower == "title" {
        return Some(1);
    }
    if lower == "subtitle" {
        return Some(2);
    }
    let rest = lower.strip_prefix("heading")?;
    rest.parse::<u8>().ok().filter(|n| (1..=6).contains(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_data_returns_error() {
        let result = extract(b"not a docx file");
        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("Failed to open DOCX"));
    }

    #[test]
    fn test_empty_data_returns_error() {
        let result = extract(b"");
        assert!(!result.success);
    }

    #[test]
    fn test_heading_level() {
        assert_eq!(heading_level(Some("Heading1")), Some(1));
        assert_eq!(heading_level(Some("Heading 3")), Some(3));
        assert_eq!(heading_level(Some("Title")), Some(1));
        assert_eq!(heading_level(Some("Normal")), None);
        assert_eq!(heading_level(Some("Heading9")), None);
    }
}
