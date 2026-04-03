use crate::ExtractionResult;
use std::io::Cursor;
use zip::ZipArchive;
use quick_xml::events::Event;
use quick_xml::reader::Reader;

pub fn extract(data: &[u8]) -> ExtractionResult {
    let cursor = Cursor::new(data);
    let mut archive = match ZipArchive::new(cursor) {
        Ok(a) => a,
        Err(e) => {
            return ExtractionResult {
                texts: vec![],
                success: false,
                error: Some(format!("Failed to open PPTX: {}", e)),
                pages: 0,
            }
        }
    };

    let mut texts: Vec<String> = Vec::new();
    let mut slide_num = 1u32;

    loop {
        let slide_path = format!("ppt/slides/slide{}.xml", slide_num);
        let xml_data = match archive.by_name(&slide_path) {
            Ok(mut file) => {
                let mut buf = Vec::new();
                std::io::Read::read_to_end(&mut file, &mut buf).ok();
                buf
            }
            Err(_) => break,
        };

        let slide_text = extract_text_from_xml(&xml_data).trim().to_string();
        texts.push(slide_text);
        slide_num += 1;
    }

    let pages = texts.len() as u32;
    let success = texts.iter().any(|t| !t.is_empty());
    ExtractionResult {
        texts: if success { texts } else { vec![] },
        success,
        error: None,
        pages,
    }
}

fn extract_text_from_xml(xml_data: &[u8]) -> String {
    let mut reader = Reader::from_reader(xml_data);
    let mut text = String::new();
    let mut in_text = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let local = e.local_name();
                if local.as_ref() == b"t" {
                    in_text = true;
                }
            }
            Ok(Event::End(ref e)) => {
                let local = e.local_name();
                if local.as_ref() == b"t" {
                    in_text = false;
                }
                if local.as_ref() == b"p" {
                    text.push('\n');
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_text {
                    if let Ok(t) = e.unescape() {
                        text.push_str(&t);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    text
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
