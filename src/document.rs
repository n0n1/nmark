use std::fs;
use std::path::{Path, PathBuf};

pub fn to_markdown(content: &str) -> String {
    let (prepared, replacements) = replace_image_tags(content);
    let markdown = html2md::parse_html(&prepared);
    restore_image_placeholders(markdown, &replacements)
}

pub fn compose_markdown(frontmatter: Option<&str>, body: &str) -> String {
    match frontmatter {
        Some(frontmatter) => format!("{frontmatter}\n{body}"),
        None => body.to_string(),
    }
}

pub fn output_path(output_dir: &Path, title: &str) -> PathBuf {
    output_dir.join(format!("{}.md", sanitize_filename(title)))
}

pub fn unique_output_path(output_dir: &Path, title: &str) -> PathBuf {
    let stem = sanitize_filename(title);
    let mut candidate = output_path(output_dir, title);
    let mut suffix = 2usize;

    while fs::exists(&candidate).unwrap_or(false) {
        candidate = output_dir.join(format!("{stem}-{suffix}.md"));
        suffix += 1;
    }

    candidate
}

fn sanitize_filename(title: &str) -> String {
    let mut sanitized = String::with_capacity(title.len());

    for ch in title.chars() {
        let safe = match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            c if c.is_control() => '-',
            c => c,
        };
        sanitized.push(safe);
    }

    let sanitized = sanitized.trim().trim_matches('.').to_string();
    if sanitized.is_empty() {
        "article".to_string()
    } else {
        sanitized
    }
}

fn replace_image_tags(content: &str) -> (String, Vec<(String, String)>) {
    let mut prepared = String::with_capacity(content.len());
    let mut replacements = Vec::new();
    let mut cursor = 0usize;
    let bytes = content.as_bytes();

    while let Some(relative_start) = find_case_insensitive(&content[cursor..], "<img") {
        let start = cursor + relative_start;
        prepared.push_str(&content[cursor..start]);

        let Some(tag_end) = find_tag_end(content, start) else {
            prepared.push_str(&content[start..]);
            cursor = content.len();
            break;
        };

        let tag = &content[start..tag_end];
        if let Some(image_markdown) = image_markdown(tag) {
            let placeholder = format!("NMARKIMAGE{}TOKEN", replacements.len());
            prepared.push_str(&placeholder);
            replacements.push((placeholder, image_markdown));
        }

        cursor = tag_end;
        if cursor >= bytes.len() {
            break;
        }
    }

    if cursor < content.len() {
        prepared.push_str(&content[cursor..]);
    }

    (prepared, replacements)
}

fn restore_image_placeholders(mut markdown: String, replacements: &[(String, String)]) -> String {
    for (placeholder, replacement) in replacements {
        markdown = markdown.replace(placeholder, replacement);
    }
    markdown
}

fn image_markdown(tag: &str) -> Option<String> {
    let src = image_src(tag)?;
    let alt = tag_attr(tag, "alt")
        .filter(|value| !value.is_empty())
        .or_else(|| tag_attr(tag, "title").filter(|value| !value.is_empty()))
        .unwrap_or_else(|| "image".to_string());

    Some(format!("![{}]({})", escape_markdown_alt(&alt), src))
}

fn image_src(tag: &str) -> Option<String> {
    for attr in ["src", "data-src", "data-original", "data-lazy-src"] {
        if let Some(value) = tag_attr(tag, attr).filter(|value| !value.is_empty()) {
            return Some(value);
        }
    }

    tag_attr(tag, "srcset")
        .and_then(|srcset| parse_srcset(&srcset))
        .filter(|value| !value.is_empty())
}

fn parse_srcset(srcset: &str) -> Option<String> {
    srcset
        .split(',')
        .filter_map(|candidate| {
            let url = candidate.split_whitespace().next()?.trim();
            (!url.is_empty()).then(|| url.to_string())
        })
        .next_back()
}

fn tag_attr(tag: &str, attr: &str) -> Option<String> {
    let attr_bytes = attr.as_bytes();
    let original = tag.as_bytes();
    let lower = tag.to_ascii_lowercase();
    let lower_bytes = lower.as_bytes();
    let mut index = 0usize;

    while index < lower_bytes.len() {
        let remaining = &lower_bytes[index..];
        let Some(relative_match) = find_bytes(remaining, attr_bytes) else {
            break;
        };
        let attr_start = index + relative_match;
        let attr_end = attr_start + attr_bytes.len();

        if attr_end >= lower_bytes.len() {
            break;
        }

        let prev = attr_start.checked_sub(1).and_then(|idx| lower_bytes.get(idx));
        if matches!(prev, Some(byte) if is_attr_name_byte(*byte)) {
            index = attr_end;
            continue;
        }

        let mut value_start = attr_end;
        while matches!(lower_bytes.get(value_start), Some(byte) if byte.is_ascii_whitespace()) {
            value_start += 1;
        }

        if lower_bytes.get(value_start) != Some(&b'=') {
            index = attr_end;
            continue;
        }

        value_start += 1;
        while matches!(lower_bytes.get(value_start), Some(byte) if byte.is_ascii_whitespace()) {
            value_start += 1;
        }

        let quote = *original.get(value_start)?;
        let (content_start, content_end) = if quote == b'"' || quote == b'\'' {
            let content_start = value_start + 1;
            let rest = &original[content_start..];
            let relative_end = rest.iter().position(|&byte| byte == quote)?;
            (content_start, content_start + relative_end)
        } else {
            let content_start = value_start;
            let relative_end = original[content_start..]
                .iter()
                .position(|byte| byte.is_ascii_whitespace() || *byte == b'>' || *byte == b'/')
                .unwrap_or(original.len() - content_start);
            (content_start, content_start + relative_end)
        };

        return Some(
            String::from_utf8_lossy(&original[content_start..content_end])
                .trim()
                .to_string(),
        );
    }

    None
}

fn escape_markdown_alt(alt: &str) -> String {
    alt.replace('\\', r"\\")
        .replace('[', r"\[")
        .replace(']', r"\]")
}

fn find_tag_end(content: &str, start: usize) -> Option<usize> {
    let bytes = content.as_bytes();
    let mut index = start;
    let mut quote = None::<u8>;

    while index < bytes.len() {
        let byte = bytes[index];
        match quote {
            Some(active) if byte == active => quote = None,
            Some(_) => {}
            None if byte == b'"' || byte == b'\'' => quote = Some(byte),
            None if byte == b'>' => return Some(index + 1),
            None => {}
        }
        index += 1;
    }

    None
}

fn find_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    let haystack = haystack.as_bytes();
    let needle = needle.as_bytes();

    haystack
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle))
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

fn is_attr_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':')
}

#[cfg(test)]
mod tests {
    use super::{output_path, to_markdown, unique_output_path};
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn sanitizes_output_filename() {
        let path = output_path(Path::new("out"), "A/B:C*D?");
        assert_eq!(path, Path::new("out").join("A-B-C-D-.md"));
    }

    #[test]
    fn creates_unique_output_filename_when_file_exists() {
        let dir = temp_dir("unique-output");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("Article.md"), "existing").unwrap();

        let path = unique_output_path(&dir, "Article");
        assert_eq!(path, dir.join("Article-2.md"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn converts_basic_image_tags_to_markdown() {
        let markdown = to_markdown(
            r#"<p>Intro</p><img src="https://example.com/image.png" alt="Diagram"><p>Outro</p>"#,
        );

        assert!(markdown.contains("Intro"));
        assert!(markdown.contains("![Diagram](https://example.com/image.png)"));
        assert!(markdown.contains("Outro"));
    }

    #[test]
    fn uses_last_srcset_candidate_when_src_is_missing() {
        let markdown = to_markdown(
            r#"<img srcset="https://example.com/small.png 780w, https://example.com/large.png 1560w">"#,
        );

        assert!(markdown.contains("![image](https://example.com/large.png)"));
    }

    #[test]
    fn preserves_images_without_alt_using_default_label() {
        let markdown = to_markdown(
            r#"<img decode="async" src="https://example.com/image.png" width="200">"#,
        );

        assert!(markdown.contains("![image](https://example.com/image.png)"));
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("rmark-{nonce}-{name}"))
    }
}
