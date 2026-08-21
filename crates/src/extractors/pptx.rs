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
        Err(e) => return ExtractionResult::failure(format!("Failed to open PPTX: {}", e)),
    };

    let mut pages: Vec<Vec<Block>> = Vec::new();
    let mut slide_num = 1u32;

    loop {
        let slide_path = format!("ppt/slides/slide{}.xml", slide_num);
        let Some(xml) = read_entry(&mut archive, &slide_path) else {
            break;
        };
        let rels_path = format!("ppt/slides/_rels/slide{}.xml.rels", slide_num);
        let links = read_rels(&mut archive, &rels_path);
        pages.push(parse_slide(&xml, &links));
        slide_num += 1;
    }

    if pages.is_empty() {
        return ExtractionResult::failure("No slides found in PPTX".to_string());
    }
    ExtractionResult::from_pages(pages)
}

fn read_entry(archive: &mut ZipArchive<Cursor<&[u8]>>, name: &str) -> Option<Vec<u8>> {
    let mut file = archive.by_name(name).ok()?;
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut buf).ok()?;
    Some(buf)
}

fn read_rels(archive: &mut ZipArchive<Cursor<&[u8]>>, name: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Some(data) = read_entry(archive, name) else {
        return map;
    };
    let mut reader = Reader::from_reader(data.as_slice());
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
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

#[derive(Default, Clone)]
struct RunProps {
    bold: bool,
    italic: bool,
    strike: bool,
    highlight: Option<String>,
    link: Option<String>,
}

#[derive(Default)]
struct TableBuilder {
    rows: Vec<Vec<Vec<Run>>>,
    row: Vec<Vec<Run>>,
    cell: Vec<Run>,
}

fn parse_slide(data: &[u8], links: &HashMap<String, String>) -> Vec<Block> {
    let mut reader = Reader::from_reader(data);
    let mut buf = Vec::new();

    let mut blocks: Vec<Block> = Vec::new();
    let mut table: Option<TableBuilder> = None;
    let mut runs: Vec<Run> = Vec::new();
    let mut rpr = RunProps::default();
    let mut in_text = false;
    let mut in_rpr = false;
    // 現在の図形がタイトルプレースホルダか
    let mut shape_is_title = false;
    let mut para_level = 0u8;
    let mut para_is_bullet = false;
    let mut list_buf: Vec<ListItem> = Vec::new();

    loop {
        let ev = reader.read_event_into(&mut buf);
        match ev {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let local = e.local_name().as_ref().to_vec();
                let attr = |n: &[u8]| -> Option<String> {
                    e.attributes().flatten().find_map(|a| {
                        (a.key.local_name().as_ref() == n)
                            .then(|| String::from_utf8_lossy(&a.value).to_string())
                    })
                };
                match local.as_slice() {
                    b"sp" => shape_is_title = false,
                    b"ph" => {
                        if let Some(t) = attr(b"type") {
                            shape_is_title = t == "title" || t == "ctrTitle";
                        }
                    }
                    b"tbl" => table = Some(TableBuilder::default()),
                    b"tr" => {
                        if let Some(t) = table.as_mut() {
                            t.row = Vec::new();
                        }
                    }
                    b"tc" => {
                        if let Some(t) = table.as_mut() {
                            t.cell = Vec::new();
                        }
                    }
                    b"p" => {
                        runs.clear();
                        para_level = 0;
                        para_is_bullet = false;
                    }
                    b"pPr" => {
                        para_level = attr(b"lvl").and_then(|v| v.parse().ok()).unwrap_or(0);
                    }
                    b"buChar" | b"buAutoNum" => para_is_bullet = true,
                    b"buNone" => para_is_bullet = false,
                    b"rPr" => {
                        in_rpr = true;
                        rpr = RunProps::default();
                        rpr.bold = attr(b"b").map_or(false, |v| v == "1" || v == "true");
                        rpr.italic = attr(b"i").map_or(false, |v| v == "1" || v == "true");
                        rpr.strike = attr(b"strike").map_or(false, |v| v.starts_with("sng"));
                    }
                    b"highlight" if in_rpr => rpr.highlight = Some("yellow".to_string()),
                    b"hlinkClick" if in_rpr => {
                        rpr.link = attr(b"id").and_then(|id| links.get(&id).cloned());
                    }
                    b"t" => in_text = true,
                    b"br" => runs.push(Run::text("\n")),
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_text {
                    if let Ok(t) = e.unescape() {
                        runs.push(Run {
                            text: t.to_string(),
                            bold: rpr.bold,
                            italic: rpr.italic,
                            strike: rpr.strike,
                            code: false,
                            highlight: rpr.highlight.clone(),
                            link: rpr.link.clone(),
                        });
                    }
                }
            }
            Ok(Event::End(ref e)) => match e.local_name().as_ref() {
                b"t" => in_text = false,
                b"rPr" => in_rpr = false,
                b"r" => rpr = RunProps::default(),
                b"p" => {
                    let merged = merge_runs(std::mem::take(&mut runs));
                    if let Some(t) = table.as_mut() {
                        if !merged.is_empty() {
                            if !t.cell.is_empty() {
                                t.cell.push(Run::text(" "));
                            }
                            t.cell.extend(merged);
                        }
                    } else if !merged.is_empty() {
                        if para_is_bullet {
                            list_buf.push(ListItem {
                                level: para_level,
                                runs: merged,
                            });
                        } else {
                            flush_list(&mut blocks, &mut list_buf);
                            if shape_is_title {
                                blocks.push(Block::Heading {
                                    level: 1,
                                    runs: merged,
                                });
                            } else {
                                blocks.push(Block::Para { runs: merged });
                            }
                        }
                    }
                }
                b"tc" => {
                    if let Some(t) = table.as_mut() {
                        let cell = merge_runs(std::mem::take(&mut t.cell));
                        t.row.push(cell);
                    }
                }
                b"tr" => {
                    if let Some(t) = table.as_mut() {
                        let row = std::mem::take(&mut t.row);
                        if !row.is_empty() {
                            t.rows.push(row);
                        }
                    }
                }
                b"tbl" => {
                    if let Some(t) = table.take() {
                        if !t.rows.is_empty() {
                            flush_list(&mut blocks, &mut list_buf);
                            blocks.push(Block::Table { rows: t.rows });
                        }
                    }
                }
                _ => {}
            },
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    flush_list(&mut blocks, &mut list_buf);
    blocks
}

fn flush_list(blocks: &mut Vec<Block>, buf: &mut Vec<ListItem>) {
    if buf.is_empty() {
        return;
    }
    blocks.push(Block::List {
        ordered: false,
        items: std::mem::take(buf),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_data_returns_error() {
        let result = extract(b"not a pptx file");
        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("Failed to open PPTX"));
    }

    #[test]
    fn test_empty_data_returns_error() {
        let result = extract(b"");
        assert!(!result.success);
    }
}
