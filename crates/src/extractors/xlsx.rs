use crate::block::{Block, Run};
use crate::ExtractionResult;
use calamine::{Reader, Xlsx};
use quick_xml::events::Event;
use quick_xml::Reader as XmlReader;
use std::collections::HashMap;
use std::io::Cursor;
use zip::ZipArchive;

/// セル座標 (例 "B3") に紐づく装飾情報
#[derive(Default, Clone)]
struct CellStyle {
    bold: bool,
    italic: bool,
    link: Option<String>,
}

pub fn extract(data: &[u8]) -> ExtractionResult {
    let cursor = Cursor::new(data);
    let mut workbook: Xlsx<_> = match Xlsx::new(cursor) {
        Ok(wb) => wb,
        Err(e) => return ExtractionResult::failure(format!("Failed to open XLSX: {}", e)),
    };

    let sheet_names: Vec<String> = workbook.sheet_names().to_vec();
    // 装飾・リンクは calamine から取れないため生 XML を併読する
    let decorations = read_decorations(data, &sheet_names);

    let mut pages: Vec<Vec<Block>> = Vec::new();

    for (idx, name) in sheet_names.iter().enumerate() {
        let Ok(range) = workbook.worksheet_range(name) else {
            continue;
        };
        let styles = decorations.get(idx).cloned().unwrap_or_default();
        let (start_row, start_col) = range.start().unwrap_or((0, 0));

        let mut blocks: Vec<Block> = vec![Block::Heading {
            level: 2,
            runs: vec![Run::text(name.clone())],
        }];

        let mut rows: Vec<Vec<Vec<Run>>> = Vec::new();
        for (r, row) in range.rows().enumerate() {
            let cells: Vec<Vec<Run>> = row
                .iter()
                .enumerate()
                .map(|(c, cell)| {
                    let text = format!("{}", cell);
                    if text.is_empty() {
                        return vec![];
                    }
                    let refname = cell_ref(start_row as usize + r, start_col as usize + c);
                    let st = styles.get(&refname).cloned().unwrap_or_default();
                    vec![Run {
                        text,
                        bold: st.bold,
                        italic: st.italic,
                        link: st.link,
                        ..Default::default()
                    }]
                })
                .collect();
            if cells.iter().any(|c| !c.is_empty()) {
                rows.push(cells);
            }
        }

        if !rows.is_empty() {
            blocks.push(Block::Table { rows });
        }
        pages.push(blocks);
    }

    if pages.is_empty() {
        return ExtractionResult::failure("No sheets found in XLSX".to_string());
    }
    ExtractionResult::from_pages(pages)
}

/// 0 始まりの行・列番号を "A1" 形式に変換
fn cell_ref(row: usize, col: usize) -> String {
    let mut name = String::new();
    let mut c = col + 1;
    while c > 0 {
        let rem = (c - 1) % 26;
        name.insert(0, (b'A' + rem as u8) as char);
        c = (c - 1) / 26;
    }
    format!("{}{}", name, row + 1)
}

/// 各シートについて セル座標 -> CellStyle を読み出す
fn read_decorations(data: &[u8], sheet_names: &[String]) -> Vec<HashMap<String, CellStyle>> {
    let cursor = Cursor::new(data);
    let Ok(mut zip) = ZipArchive::new(cursor) else {
        return vec![];
    };

    let bold_italic_by_xf = read_styles(&mut zip);
    let sheet_paths = read_sheet_paths(&mut zip, sheet_names);

    sheet_paths
        .iter()
        .map(|path| match path {
            Some(p) => read_sheet(&mut zip, p, &bold_italic_by_xf),
            None => HashMap::new(),
        })
        .collect()
}

fn read_entry(zip: &mut ZipArchive<Cursor<&[u8]>>, name: &str) -> Option<Vec<u8>> {
    let mut f = zip.by_name(name).ok()?;
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut buf).ok()?;
    Some(buf)
}

/// styles.xml から cellXfs のインデックス -> (bold, italic)
fn read_styles(zip: &mut ZipArchive<Cursor<&[u8]>>) -> Vec<(bool, bool)> {
    let Some(data) = read_entry(zip, "xl/styles.xml") else {
        return vec![];
    };
    let mut fonts: Vec<(bool, bool)> = Vec::new();
    let mut xfs: Vec<usize> = Vec::new();

    let mut reader = XmlReader::from_reader(data.as_slice());
    let mut buf = Vec::new();
    let mut in_fonts = false;
    let mut in_cell_xfs = false;
    let mut cur = (false, false);

    loop {
        let ev = reader.read_event_into(&mut buf);
        match ev {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let is_empty = matches!(ev, Ok(Event::Empty(_)));
                let local = e.local_name().as_ref().to_vec();
                let attr = |n: &[u8]| -> Option<String> {
                    e.attributes().flatten().find_map(|a| {
                        (a.key.local_name().as_ref() == n)
                            .then(|| String::from_utf8_lossy(&a.value).to_string())
                    })
                };
                match local.as_slice() {
                    b"fonts" => in_fonts = true,
                    b"cellXfs" => in_cell_xfs = true,
                    b"font" if in_fonts => {
                        cur = (false, false);
                        // <font/> は End イベントが来ないのでここで確定させる
                        if is_empty {
                            fonts.push(cur);
                        }
                    }
                    b"b" if in_fonts => cur.0 = attr(b"val").map_or(true, |v| v != "0"),
                    b"i" if in_fonts => cur.1 = attr(b"val").map_or(true, |v| v != "0"),
                    b"xf" if in_cell_xfs => {
                        let id = attr(b"fontId").and_then(|v| v.parse().ok()).unwrap_or(0);
                        xfs.push(id);
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => match e.local_name().as_ref() {
                b"font" if in_fonts => fonts.push(cur),
                b"fonts" => in_fonts = false,
                b"cellXfs" => in_cell_xfs = false,
                _ => {}
            },
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    xfs.iter()
        .map(|&fid| fonts.get(fid).copied().unwrap_or((false, false)))
        .collect()
}

/// workbook.xml + rels から シート順 -> xl/worksheets/*.xml のパス
fn read_sheet_paths(
    zip: &mut ZipArchive<Cursor<&[u8]>>,
    sheet_names: &[String],
) -> Vec<Option<String>> {
    let mut name_to_rid: Vec<(String, String)> = Vec::new();
    if let Some(data) = read_entry(zip, "xl/workbook.xml") {
        let mut reader = XmlReader::from_reader(data.as_slice());
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    if e.local_name().as_ref() == b"sheet" {
                        let mut n = String::new();
                        let mut rid = String::new();
                        for a in e.attributes().flatten() {
                            let v = String::from_utf8_lossy(&a.value).to_string();
                            match a.key.local_name().as_ref() {
                                b"name" => n = v,
                                b"id" => rid = v,
                                _ => {}
                            }
                        }
                        name_to_rid.push((n, rid));
                    }
                }
                Ok(Event::Eof) | Err(_) => break,
                _ => {}
            }
            buf.clear();
        }
    }

    let mut rid_to_target: HashMap<String, String> = HashMap::new();
    if let Some(data) = read_entry(zip, "xl/_rels/workbook.xml.rels") {
        let mut reader = XmlReader::from_reader(data.as_slice());
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    if e.local_name().as_ref() == b"Relationship" {
                        let mut id = String::new();
                        let mut target = String::new();
                        for a in e.attributes().flatten() {
                            let v = String::from_utf8_lossy(&a.value).to_string();
                            match a.key.local_name().as_ref() {
                                b"Id" => id = v,
                                b"Target" => target = v,
                                _ => {}
                            }
                        }
                        rid_to_target.insert(id, target);
                    }
                }
                Ok(Event::Eof) | Err(_) => break,
                _ => {}
            }
            buf.clear();
        }
    }

    sheet_names
        .iter()
        .map(|name| {
            let rid = name_to_rid
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, r)| r.clone())?;
            let target = rid_to_target.get(&rid)?;
            let t = target.trim_start_matches('/');
            Some(if t.starts_with("xl/") {
                t.to_string()
            } else {
                format!("xl/{}", t)
            })
        })
        .collect()
}

/// 1シート分の セル座標 -> CellStyle
fn read_sheet(
    zip: &mut ZipArchive<Cursor<&[u8]>>,
    path: &str,
    xf_styles: &[(bool, bool)],
) -> HashMap<String, CellStyle> {
    let mut map: HashMap<String, CellStyle> = HashMap::new();
    let Some(data) = read_entry(zip, path) else {
        return map;
    };

    // シート個別 rels (ハイパーリンクの外部URL)
    let rels_path = path.replace("worksheets/", "worksheets/_rels/") + ".rels";
    let mut rid_to_url: HashMap<String, String> = HashMap::new();
    if let Some(rd) = read_entry(zip, &rels_path) {
        let mut reader = XmlReader::from_reader(rd.as_slice());
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    if e.local_name().as_ref() == b"Relationship" {
                        let mut id = String::new();
                        let mut target = String::new();
                        for a in e.attributes().flatten() {
                            let v = String::from_utf8_lossy(&a.value).to_string();
                            match a.key.local_name().as_ref() {
                                b"Id" => id = v,
                                b"Target" => target = v,
                                _ => {}
                            }
                        }
                        rid_to_url.insert(id, target);
                    }
                }
                Ok(Event::Eof) | Err(_) => break,
                _ => {}
            }
            buf.clear();
        }
    }

    let mut reader = XmlReader::from_reader(data.as_slice());
    let mut buf = Vec::new();
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
                    b"c" => {
                        if let Some(r) = attr(b"r") {
                            let s: usize = attr(b"s").and_then(|v| v.parse().ok()).unwrap_or(0);
                            let (bold, italic) = xf_styles.get(s).copied().unwrap_or((false, false));
                            if bold || italic {
                                let entry = map.entry(r).or_default();
                                entry.bold = bold;
                                entry.italic = italic;
                            }
                        }
                    }
                    b"hyperlink" => {
                        let Some(refs) = attr(b"ref") else { continue };
                        let url = attr(b"id")
                            .and_then(|id| rid_to_url.get(&id).cloned())
                            .or_else(|| attr(b"location"));
                        if let Some(u) = url {
                            // "A1" 単体のみ対応 (範囲指定は先頭セルに付与)
                            let first = refs.split(':').next().unwrap_or(&refs).to_string();
                            map.entry(first).or_default().link = Some(u);
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
    map
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

    #[test]
    fn test_cell_ref() {
        assert_eq!(cell_ref(0, 0), "A1");
        assert_eq!(cell_ref(2, 1), "B3");
        assert_eq!(cell_ref(0, 25), "Z1");
        assert_eq!(cell_ref(0, 26), "AA1");
    }
}
