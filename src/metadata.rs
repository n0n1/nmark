use std::error::Error;
use std::fmt;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ArticleMetadata {
    pub author: Option<String>,
    pub tags: Vec<String>,
}

pub type MetadataResult<T> = Result<T, MetadataError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadataError {
    UnterminatedMetaTag,
    UnterminatedAttributeValue { attr: &'static str },
}

pub fn extract_metadata(html: &str) -> ArticleMetadata {
    extract_metadata_strict(html).unwrap_or_else(|_| extract_metadata_lossy(html))
}

fn extract_metadata_strict(html: &str) -> MetadataResult<ArticleMetadata> {
    extract_metadata_with(html, iter_meta_tags_strict)
}

fn extract_metadata_lossy(html: &str) -> ArticleMetadata {
    let mut metadata = ArticleMetadata::default();

    for meta_tag in iter_meta_tags_lossy(html) {
        let _ = apply_meta_tag(meta_tag, &mut metadata);
    }

    metadata
}

fn extract_metadata_with<'a, F>(html: &'a str, iter_meta_tags: F) -> MetadataResult<ArticleMetadata>
where
    F: Fn(&'a str) -> MetadataResult<Vec<&'a str>>,
{
    let mut metadata = ArticleMetadata::default();

    for meta_tag in iter_meta_tags(html)? {
        apply_meta_tag(meta_tag, &mut metadata)?;
    }

    Ok(metadata)
}

fn iter_meta_tags_strict(html: &str) -> MetadataResult<Vec<&str>> {
    let mut tags = Vec::new();
    let mut cursor = 0;

    while let Some(start) = find_meta_tag_start(html, cursor) {
        let Some(end) = html[start..].find('>') else {
            return Err(MetadataError::UnterminatedMetaTag);
        };
        let end = start + end + 1;
        tags.push(&html[start..end]);
        cursor = end;
    }

    Ok(tags)
}

fn iter_meta_tags_lossy(html: &str) -> Vec<&str> {
    let mut tags = Vec::new();
    let mut cursor = 0;

    while let Some(start) = find_meta_tag_start(html, cursor) {
        let Some(end) = html[start..].find('>') else {
            break;
        };
        let end = start + end + 1;
        tags.push(&html[start..end]);
        cursor = end;
    }

    tags
}

fn find_meta_tag_start(html: &str, from: usize) -> Option<usize> {
    let bytes = html.as_bytes();
    let mut index = from;

    while index + 5 <= bytes.len() {
        if bytes[index] == b'<'
            && bytes[index + 1].eq_ignore_ascii_case(&b'm')
            && bytes[index + 2].eq_ignore_ascii_case(&b'e')
            && bytes[index + 3].eq_ignore_ascii_case(&b't')
            && bytes[index + 4].eq_ignore_ascii_case(&b'a')
            && (index + 5 == bytes.len()
                || bytes[index + 5].is_ascii_whitespace()
                || matches!(bytes[index + 5], b'/' | b'>'))
        {
            return Some(index);
        }
        index += 1;
    }

    None
}

fn meta_attr(tag: &str, attr: &'static str) -> MetadataResult<Option<String>> {
    let lower = tag.to_ascii_lowercase();
    let attr_bytes = attr.as_bytes();
    let bytes = lower.as_bytes();
    let original = tag.as_bytes();
    let mut index = 0;

    while index + attr_bytes.len() <= bytes.len() {
        if &bytes[index..index + attr_bytes.len()] == attr_bytes
            && is_attr_name_start(bytes, index)
            && is_attr_name_end(bytes, index + attr_bytes.len())
        {
            let mut value_start = index + attr_bytes.len();
            while value_start < bytes.len() && bytes[value_start].is_ascii_whitespace() {
                value_start += 1;
            }
            if value_start >= bytes.len() || bytes[value_start] != b'=' {
                index += attr_bytes.len();
                continue;
            }
            value_start += 1;
            while value_start < bytes.len() && bytes[value_start].is_ascii_whitespace() {
                value_start += 1;
            }
            if value_start >= bytes.len() {
                return Ok(None);
            }

            let quote = original[value_start];
            let (content_start, content_end) = if quote == b'"' || quote == b'\'' {
                let content_start = value_start + 1;
                let rest = &original[content_start..];
                let Some(relative_end) = rest.iter().position(|byte| *byte == quote) else {
                    return Err(MetadataError::UnterminatedAttributeValue { attr });
                };
                (content_start, content_start + relative_end)
            } else {
                let content_start = value_start;
                let relative_end = original[content_start..]
                    .iter()
                    .position(|byte| byte.is_ascii_whitespace() || *byte == b'>')
                    .unwrap_or(original.len() - content_start);
                (content_start, content_start + relative_end)
            };

            let value = String::from_utf8_lossy(&original[content_start..content_end])
                .trim()
                .to_string();
            return Ok((!value.is_empty()).then_some(value));
        }

        index += 1;
    }

    Ok(None)
}

fn apply_meta_tag(tag: &str, metadata: &mut ArticleMetadata) -> MetadataResult<()> {
    let content = match meta_attr(tag, "content")? {
        Some(content) if !content.trim().is_empty() => content,
        _ => return Ok(()),
    };

    let name = meta_attr(tag, "name")?;
    let property = meta_attr(tag, "property")?;
    let itemprop = meta_attr(tag, "itemprop")?;

    if metadata.author.is_none()
        && is_author_meta(name.as_deref(), property.as_deref(), itemprop.as_deref())
    {
        metadata.author = Some(content.clone());
    }

    if is_tag_meta(name.as_deref(), property.as_deref(), itemprop.as_deref()) {
        push_tags(&mut metadata.tags, &content);
        return Ok(());
    }

    if metadata.tags.is_empty()
        && is_keywords_meta(name.as_deref(), property.as_deref(), itemprop.as_deref())
    {
        push_tags(&mut metadata.tags, &content);
    }

    Ok(())
}

fn is_attr_name_start(bytes: &[u8], index: usize) -> bool {
    if index == 0 {
        return true;
    }

    matches!(bytes[index - 1], b'<' | b'>' | b'/' | b' ' | b'\n' | b'\r' | b'\t')
}

fn is_attr_name_end(bytes: &[u8], index: usize) -> bool {
    if index >= bytes.len() {
        return true;
    }

    matches!(bytes[index], b'=' | b' ' | b'\n' | b'\r' | b'\t')
}

fn is_author_meta(name: Option<&str>, property: Option<&str>, itemprop: Option<&str>) -> bool {
    matches_ignore_ascii_case(name, "author")
        || matches_ignore_ascii_case(name, "parsely-author")
        || matches_ignore_ascii_case(name, "twitter:creator")
        || matches_ignore_ascii_case(property, "author")
        || matches_ignore_ascii_case(property, "article:author")
        || matches_ignore_ascii_case(property, "og:article:author")
        || matches_ignore_ascii_case(itemprop, "author")
}

fn is_tag_meta(name: Option<&str>, property: Option<&str>, itemprop: Option<&str>) -> bool {
    matches_ignore_ascii_case(property, "article:tag")
        || matches_ignore_ascii_case(name, "article:tag")
        || matches_ignore_ascii_case(name, "parsely-tags")
        || matches_ignore_ascii_case(name, "news_keywords")
        || matches_ignore_ascii_case(itemprop, "keywords")
}

fn is_keywords_meta(name: Option<&str>, property: Option<&str>, itemprop: Option<&str>) -> bool {
    matches_ignore_ascii_case(name, "keywords")
        || matches_ignore_ascii_case(property, "keywords")
        || matches_ignore_ascii_case(itemprop, "keywords")
}

fn matches_ignore_ascii_case(value: Option<&str>, expected: &str) -> bool {
    value.is_some_and(|value| value.eq_ignore_ascii_case(expected))
}

fn push_tags(tags: &mut Vec<String>, value: &str) {
    for candidate in split_tag_candidates(value) {
        push_tag(tags, candidate);
    }
}

fn split_tag_candidates(value: &str) -> impl Iterator<Item = &str> {
    value
        .split([',', ';'])
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
}

fn push_tag(tags: &mut Vec<String>, value: &str) {
    let tag = value.trim();
    if tag.is_empty() || tags.iter().any(|existing| existing.eq_ignore_ascii_case(tag)) {
        return;
    }

    tags.push(tag.to_string());
}

impl fmt::Display for MetadataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetadataError::UnterminatedMetaTag => f.write_str("unterminated <meta> tag"),
            MetadataError::UnterminatedAttributeValue { attr } => {
                write!(f, "unterminated attribute value for `{attr}`")
            }
        }
    }
}

impl Error for MetadataError {}

#[cfg(test)]
mod tests {
    use super::{extract_metadata, extract_metadata_strict, find_meta_tag_start, MetadataError};

    #[test]
    fn extracts_author_and_tags() {
        let html = r#"
            <html>
                <head>
                    <meta name="author" content="Jane Doe">
                    <meta property="article:tag" content="rust">
                    <meta property="article:tag" content="cli">
                </head>
            </html>
        "#;

        let metadata = extract_metadata(html);
        assert_eq!(metadata.author.as_deref(), Some("Jane Doe"));
        assert_eq!(metadata.tags, vec!["rust", "cli"]);
    }

    #[test]
    fn falls_back_to_keywords() {
        let html = r#"<meta name="keywords" content="rust, cli, markdown">"#;
        let metadata = extract_metadata(html);
        assert_eq!(metadata.tags, vec!["rust", "cli", "markdown"]);
    }

    #[test]
    fn strict_rejects_unterminated_meta_tag() {
        let html = r#"<meta name="keywords" content="rust""#;
        let error = extract_metadata_strict(html).unwrap_err();
        assert_eq!(error, MetadataError::UnterminatedMetaTag);
    }

    #[test]
    fn strict_rejects_unterminated_attribute_value() {
        let html = r#"<meta name="keywords" content="rust>"#;
        let error = extract_metadata_strict(html).unwrap_err();
        assert_eq!(
            error,
            MetadataError::UnterminatedAttributeValue { attr: "content" }
        );
    }

    #[test]
    fn lossy_mode_ignores_broken_meta() {
        let html = r#"
            <meta name="author" content="Jane Doe">
            <meta name="keywords" content="rust, cli
        "#;
        let metadata = extract_metadata(html);
        assert_eq!(metadata.author.as_deref(), Some("Jane Doe"));
        assert!(metadata.tags.is_empty());
    }

    #[test]
    fn extracts_extended_tag_and_author_sources() {
        let html = r#"
            <META NAME="parsely-author" CONTENT="Jane Doe">
            <meta name="parsely-tags" content="rust; cli; markdown">
            <meta name="news_keywords" content="rust, systems">
        "#;
        let metadata = extract_metadata(html);
        assert_eq!(metadata.author.as_deref(), Some("Jane Doe"));
        assert_eq!(metadata.tags, vec!["rust", "cli", "markdown", "systems"]);
    }

    #[test]
    fn finds_uppercase_meta_tags() {
        let html = r#"<META NAME="author" CONTENT="Jane Doe">"#;
        assert_eq!(find_meta_tag_start(html, 0), Some(0));
    }
}
