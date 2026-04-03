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
                error: Some(format!("Failed to open DOCX: {}", e)),
                pages: 0,
            }
        }
    };

    let xml_data = match archive.by_name("word/document.xml") {
        Ok(mut file) => {
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut file, &mut buf).ok();
            buf
        }
        Err(e) => {
            return ExtractionResult {
                texts: vec![],
                success: false,
                error: Some(format!("No document.xml found: {}", e)),
                pages: 0,
            }
        }
    };

    let mut reader = Reader::from_reader(xml_data.as_slice());
    let mut text = String::new();
    let mut in_text = false;
    let mut in_paragraph = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let local = e.local_name();
                if local.as_ref() == b"p" {
                    in_paragraph = true;
                } else if local.as_ref() == b"t" {
                    in_text = true;
                }
            }
            Ok(Event::End(ref e)) => {
                let local = e.local_name();
                if local.as_ref() == b"p" {
                    if in_paragraph {
                        text.push('\n');
                    }
                    in_paragraph = false;
                } else if local.as_ref() == b"t" {
                    in_text = false;
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
            Err(e) => {
                return ExtractionResult {
                    texts: vec![],
                    success: false,
                    error: Some(format!("XML parse error: {}", e)),
                    pages: 0,
                }
            }
            _ => {}
        }
        buf.clear();
    }

    let trimmed = text.trim().to_string();
    let success = !trimmed.is_empty();
    ExtractionResult {
        texts: if success { vec![trimmed] } else { vec![] },
        success,
        error: None,
        pages: if success { 1 } else { 0 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_data_returns_error() {
        let result = extract(b"not a zip file");
        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("Failed to open DOCX"));
    }

    #[test]
    fn test_empty_data_returns_error() {
        let result = extract(b"");
        assert!(!result.success);
    }
}
