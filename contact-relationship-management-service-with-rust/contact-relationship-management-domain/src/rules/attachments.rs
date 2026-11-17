//! File attachments (CRM-R42): which files are accepted, how a name is cleaned, and how a
//! download is named. Pure and DB-free.
//!
//! The allow-list is by extension, and the stored content type comes from it, never from the
//! request. For formats with a recognisable start (PDF, PNG, JPEG, GIF, WebP, and the Office
//! formats, which are ZIP files) the first bytes must agree with the extension.

/// Most files on one record.
pub const MAX_PER_RECORD: i64 = 20;
/// Longest name kept.
pub const MAX_NAME: usize = 120;

/// An accepted kind of file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kind {
    /// Lower-case extension, no dot.
    pub extension: &'static str,
    /// The content type it is served with.
    pub content_type: &'static str,
    /// The bytes it must start with, when it has a recognisable start.
    magic: &'static [&'static [u8]],
}

const ZIP: &[&[u8]] = &[b"PK\x03\x04"];
const NONE: &[&[u8]] = &[];

/// The accepted kinds. Archives, scripts, executables, HTML and SVG are not on it.
pub const KINDS: &[Kind] = &[
    Kind {
        extension: "pdf",
        content_type: "application/pdf",
        magic: &[b"%PDF-"],
    },
    Kind {
        extension: "png",
        content_type: "image/png",
        magic: &[b"\x89PNG\r\n\x1a\n"],
    },
    Kind {
        extension: "jpg",
        content_type: "image/jpeg",
        magic: &[b"\xff\xd8\xff"],
    },
    Kind {
        extension: "jpeg",
        content_type: "image/jpeg",
        magic: &[b"\xff\xd8\xff"],
    },
    Kind {
        extension: "gif",
        content_type: "image/gif",
        magic: &[b"GIF87a", b"GIF89a"],
    },
    Kind {
        extension: "webp",
        content_type: "image/webp",
        magic: &[b"RIFF"],
    },
    Kind {
        extension: "txt",
        content_type: "text/plain; charset=utf-8",
        magic: NONE,
    },
    Kind {
        extension: "csv",
        content_type: "text/csv; charset=utf-8",
        magic: NONE,
    },
    Kind {
        extension: "md",
        content_type: "text/plain; charset=utf-8",
        magic: NONE,
    },
    Kind {
        extension: "docx",
        content_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        magic: ZIP,
    },
    Kind {
        extension: "xlsx",
        content_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        magic: ZIP,
    },
    Kind {
        extension: "pptx",
        content_type: "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        magic: ZIP,
    },
    Kind {
        extension: "odt",
        content_type: "application/vnd.oasis.opendocument.text",
        magic: ZIP,
    },
    Kind {
        extension: "ods",
        content_type: "application/vnd.oasis.opendocument.spreadsheet",
        magic: ZIP,
    },
];

/// A name made safe to store and to put in a header: no directory part, no control characters,
/// no quotes or slashes, at most [`MAX_NAME`] characters. `None` if nothing is left.
#[must_use]
pub fn clean_name(raw: &str) -> Option<String> {
    let base = raw.rsplit(['/', '\\']).next().unwrap_or(raw);
    let cleaned: String = base
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '"' | '<' | '>' | ':' | '|' | '*' | '?' | ';') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_start_matches('.').to_string();
    let cleaned: String = cleaned.chars().take(MAX_NAME).collect();
    (!cleaned.is_empty()).then_some(cleaned)
}

/// The kind of a file by its name's extension.
#[must_use]
pub fn kind_of(name: &str) -> Option<&'static Kind> {
    let (_, extension) = name.rsplit_once('.')?;
    let extension = extension.to_ascii_lowercase();
    KINDS.iter().find(|k| k.extension == extension)
}

/// A file checked: the cleaned name and its kind, or the reason it is refused.
///
/// # Errors
///
/// The reason, in words.
pub fn check(
    raw_name: &str,
    bytes: &[u8],
    max_bytes: u64,
) -> Result<(String, &'static Kind), String> {
    let name = clean_name(raw_name).ok_or("the file needs a name")?;
    let kind = kind_of(&name).ok_or_else(|| {
        format!(
            "that kind of file is not accepted; accepted: {}",
            KINDS
                .iter()
                .map(|k| k.extension)
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;
    if bytes.is_empty() {
        return Err("the file is empty".to_string());
    }
    if bytes.len() as u64 > max_bytes {
        return Err(format!("the file is larger than {max_bytes} bytes"));
    }
    if !kind.magic.is_empty() && !kind.magic.iter().any(|m| bytes.starts_with(m)) {
        return Err(format!(
            "the file does not look like a .{} file",
            kind.extension
        ));
    }
    Ok((name, kind))
}

/// The `Content-Disposition` for a download: always an attachment, with an ASCII fallback name and
/// the real one percent-encoded (RFC 6266).
#[must_use]
pub fn disposition(name: &str) -> String {
    use std::fmt::Write as _;
    let ascii: String = name
        .chars()
        .map(|c| {
            if c.is_ascii() && !c.is_ascii_control() && c != '\\' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let mut encoded = String::new();
    for byte in name.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    format!("attachment; filename=\"{ascii}\"; filename*=UTF-8''{encoded}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_loses_its_path_and_its_dangerous_characters() {
        assert_eq!(clean_name("../../etc/passwd").as_deref(), Some("passwd"));
        assert_eq!(clean_name("C:\\docs\\a\"b.pdf").as_deref(), Some("a_b.pdf"));
        assert_eq!(clean_name(".hidden.txt").as_deref(), Some("hidden.txt"));
        assert_eq!(
            clean_name("line\nbreak.txt").as_deref(),
            Some("line_break.txt")
        );
        assert_eq!(clean_name("   "), None);
        assert_eq!(clean_name("/"), None);
        assert_eq!(
            clean_name(&"a".repeat(300)).unwrap().chars().count(),
            MAX_NAME
        );
    }

    #[test]
    fn only_listed_extensions_are_accepted() {
        assert!(kind_of("a.PDF").is_some());
        assert!(kind_of("a.exe").is_none());
        assert!(kind_of("a.html").is_none());
        assert!(kind_of("a.svg").is_none());
        assert!(kind_of("a.zip").is_none());
        assert!(kind_of("noextension").is_none());
    }

    #[test]
    fn the_start_of_the_file_must_agree_with_its_extension() {
        assert!(check("a.pdf", b"%PDF-1.7 ...", 100).is_ok());
        assert!(check("a.pdf", b"<script>alert(1)</script>", 100).is_err());
        assert!(check("a.png", b"\x89PNG\r\n\x1a\nrest", 100).is_ok());
        assert!(check("a.docx", b"PK\x03\x04...", 100).is_ok());
        assert!(check("a.docx", b"not a zip", 100).is_err());
        assert!(check("notes.txt", b"anything at all", 100).is_ok());
    }

    #[test]
    fn size_and_emptiness_are_checked() {
        assert!(check("a.txt", b"", 100).is_err());
        assert!(check("a.txt", &[b'x'; 101], 100).is_err());
        assert!(check("a.txt", &[b'x'; 100], 100).is_ok());
    }

    #[test]
    fn a_download_is_always_an_attachment_with_a_safe_name() {
        let header = disposition("Ré\"sumé.pdf");
        assert!(header.starts_with("attachment; filename=\"R_"));
        assert!(header.contains("filename*=UTF-8''R%C3%A9%22sum%C3%A9.pdf"));
        assert!(!header.contains('\n'));
    }
}
