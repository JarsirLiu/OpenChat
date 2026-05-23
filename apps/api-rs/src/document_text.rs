use std::io::{Cursor, Read};

use quick_xml::{events::Event, Reader};

const MAX_EXTRACTED_TEXT_CHARS: usize = 60_000;

pub fn extract_supported_document_text(
    bytes: &[u8],
    mime_type: &str,
    file_name: &str,
) -> Result<Option<String>, String> {
    let normalized_mime = mime_type.to_ascii_lowercase();
    let lower_name = file_name.to_ascii_lowercase();

    let extracted = if normalized_mime == "application/pdf" || lower_name.ends_with(".pdf") {
        Some(extract_pdf_text(bytes)?)
    } else if normalized_mime
        == "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        || lower_name.ends_with(".docx")
    {
        Some(extract_docx_text(bytes)?)
    } else if normalized_mime.starts_with("text/")
        || matches!(
            normalized_mime.as_str(),
            "application/json" | "application/xml" | "application/x-ndjson"
        )
        || lower_name.ends_with(".txt")
        || lower_name.ends_with(".md")
        || lower_name.ends_with(".markdown")
    {
        Some(extract_utf8_text(bytes)?)
    } else {
        None
    };

    Ok(extracted.and_then(|text| {
        let normalized = normalize_text(text.as_str());
        if normalized.is_empty() {
            None
        } else {
            Some(truncate_chars(normalized.as_str(), MAX_EXTRACTED_TEXT_CHARS))
        }
    }))
}

fn extract_utf8_text(bytes: &[u8]) -> Result<String, String> {
    std::str::from_utf8(bytes)
        .map(str::to_string)
        .map_err(|_| "文件不是有效的 UTF-8 文本".to_string())
}

fn extract_pdf_text(bytes: &[u8]) -> Result<String, String> {
    let document = lopdf::Document::load_mem(bytes)
        .map_err(|error| format!("PDF 解析失败：{error}"))?;
    let pages = document.get_pages().keys().copied().collect::<Vec<_>>();
    document
        .extract_text(pages.as_slice())
        .map_err(|error| format!("PDF 文本提取失败：{error}"))
}

fn extract_docx_text(bytes: &[u8]) -> Result<String, String> {
    let reader = Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|error| format!("DOCX 解析失败：{error}"))?;
    let mut document_xml = String::new();
    archive
        .by_name("word/document.xml")
        .map_err(|_| "DOCX 缺少 word/document.xml".to_string())?
        .read_to_string(&mut document_xml)
        .map_err(|error| format!("DOCX 正文读取失败：{error}"))?;

    let mut reader = Reader::from_str(document_xml.as_str());
    reader.config_mut().trim_text(true);
    let mut text = String::new();
    let mut in_text = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => {
                let name = event.name();
                if name.as_ref() == b"w:t" {
                    in_text = true;
                } else if name.as_ref() == b"w:p" && !text.ends_with('\n') {
                    text.push('\n');
                }
            }
            Ok(Event::End(event)) => {
                if event.name().as_ref() == b"w:t" {
                    in_text = false;
                    text.push(' ');
                }
            }
            Ok(Event::Text(event)) if in_text => {
                text.push_str(
                    event
                        .unescape()
                        .map_err(|error| format!("DOCX 文本解码失败：{error}"))?
                        .as_ref(),
                );
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(format!("DOCX XML 解析失败：{error}")),
            _ => {}
        }
    }

    Ok(text)
}

fn normalize_text(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}\n\n[内容过长，已截断]")
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::extract_supported_document_text;

    #[test]
    fn extracts_plain_text_uploads() {
        let extracted =
            extract_supported_document_text(b" hello \n\n world ", "text/plain", "note.txt")
                .expect("text extraction should succeed")
                .expect("text should be extracted");

        assert_eq!(extracted, "hello\nworld");
    }

    #[test]
    fn ignores_unsupported_binary_uploads() {
        let extracted =
            extract_supported_document_text(b"abc", "application/octet-stream", "file.bin")
                .expect("unsupported files should not fail");

        assert!(extracted.is_none());
    }
}
