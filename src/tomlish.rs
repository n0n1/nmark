use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TomlValue {
    String(String),
    Bool(bool),
    Integer(i64),
    StringArray(Vec<String>),
}

pub type TomlishResult<T> = Result<T, TomlishError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TomlishError {
    InvalidSection(String),
    InvalidAssignment(String),
    UnterminatedArray(String),
    UnsupportedValue(String),
    UnterminatedString(String),
}

pub fn parse(text: &str) -> TomlishResult<HashMap<String, TomlValue>> {
    let mut entries = HashMap::new();
    let mut section = String::new();
    let mut pending_key = None::<String>;
    let mut pending_value = String::new();

    for raw_line in text.lines() {
        let line = strip_comments(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        if let Some(key) = pending_key.as_ref() {
            if !pending_value.is_empty() {
                pending_value.push('\n');
            }
            pending_value.push_str(line);
            if !array_is_closed(&pending_value) {
                continue;
            }

            let value = parse_value(&pending_value)?;
            entries.insert(key.clone(), value);
            pending_key = None;
            pending_value.clear();
            continue;
        }

        if line.starts_with('[') {
            if !line.ends_with(']') || line.len() < 3 {
                return Err(TomlishError::InvalidSection(line.to_string()));
            }
            section = line[1..line.len() - 1].trim().to_string();
            continue;
        }

        let Some((key, value)) = split_assignment(line) else {
            return Err(TomlishError::InvalidAssignment(line.to_string()));
        };

        let full_key = if section.is_empty() {
            key.to_string()
        } else {
            format!("{section}.{key}")
        };

        if value.trim_start().starts_with('[') && !array_is_closed(value) {
            pending_key = Some(full_key);
            pending_value = value.to_string();
            continue;
        }

        entries.insert(full_key, parse_value(value)?);
    }

    if let Some(key) = pending_key {
        return Err(TomlishError::UnterminatedArray(key));
    }

    Ok(entries)
}

fn split_assignment(line: &str) -> Option<(&str, &str)> {
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for (index, ch) in line.char_indices() {
        match ch {
            '\\' if in_double => escaped = !escaped,
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single && !escaped => in_double = !in_double,
            '=' if !in_single && !in_double => {
                let key = line[..index].trim();
                let value = line[index + 1..].trim();
                return (!key.is_empty()).then_some((key, value));
            }
            _ => escaped = false,
        }
    }

    None
}

fn parse_value(value: &str) -> TomlishResult<TomlValue> {
    let value = value.trim();
    if value.eq("true") {
        return Ok(TomlValue::Bool(true));
    }
    if value.eq("false") {
        return Ok(TomlValue::Bool(false));
    }
    if let Some(value) = parse_integer(value) {
        return Ok(TomlValue::Integer(value));
    }
    if value.starts_with('[') {
        return parse_array(value).map(TomlValue::StringArray);
    }
    parse_string_like(value).map(TomlValue::String)
}

fn parse_integer(value: &str) -> Option<i64> {
    let digits = value.strip_prefix('-').unwrap_or(value);
    if digits.is_empty() || !digits.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

fn parse_array(value: &str) -> TomlishResult<Vec<String>> {
    if !value.starts_with('[') || !value.ends_with(']') {
        return Err(TomlishError::UnsupportedValue(value.to_string()));
    }

    let inner = &value[1..value.len() - 1];
    let mut items = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for ch in inner.chars() {
        match ch {
            '\\' if in_double => {
                escaped = !escaped;
                current.push(ch);
            }
            '\'' if !in_double => {
                in_single = !in_single;
                current.push(ch);
            }
            '"' if !in_single && !escaped => {
                in_double = !in_double;
                current.push(ch);
            }
            ',' if !in_single && !in_double => {
                let item = current.trim();
                if !item.is_empty() {
                    items.push(parse_string_like(item)?);
                }
                current.clear();
            }
            _ => {
                escaped = false;
                current.push(ch);
            }
        }
    }

    let last = current.trim();
    if !last.is_empty() {
        items.push(parse_string_like(last)?);
    }

    Ok(items)
}

fn parse_string_like(value: &str) -> TomlishResult<String> {
    let value = value.trim();
    if let Some(quote) = value.chars().next().filter(|quote| *quote == '"' || *quote == '\'') {
        if !value.ends_with(quote) || value.len() < 2 {
            return Err(TomlishError::UnterminatedString(value.to_string()));
        }
        let inner = &value[1..value.len() - 1];
        if quote == '"' {
            let mut out = String::with_capacity(inner.len());
            let mut chars = inner.chars();
            while let Some(ch) = chars.next() {
                if ch == '\\' {
                    let Some(next) = chars.next() else {
                        return Err(TomlishError::UnterminatedString(value.to_string()));
                    };
                    match next {
                        '\\' => out.push('\\'),
                        '"' => out.push('"'),
                        'n' => out.push('\n'),
                        't' => out.push('\t'),
                        other => out.push(other),
                    }
                } else {
                    out.push(ch);
                }
            }
            Ok(out)
        } else {
            Ok(inner.to_string())
        }
    } else {
        Ok(value.to_string())
    }
}

fn strip_comments(line: &str) -> &str {
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for (index, ch) in line.char_indices() {
        match ch {
            '\\' if in_double => escaped = !escaped,
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single && !escaped => in_double = !in_double,
            '#' if !in_single && !in_double => return &line[..index],
            _ => escaped = false,
        }
    }

    line
}

fn array_is_closed(value: &str) -> bool {
    let mut depth = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for ch in value.chars() {
        match ch {
            '\\' if in_double => escaped = !escaped,
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single && !escaped => in_double = !in_double,
            '[' if !in_single && !in_double => depth += 1,
            ']' if !in_single && !in_double => depth = depth.saturating_sub(1),
            _ => escaped = false,
        }
    }

    depth == 0
}

impl fmt::Display for TomlishError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TomlishError::InvalidSection(line) => write!(f, "invalid TOML section: {line}"),
            TomlishError::InvalidAssignment(line) => write!(f, "invalid TOML assignment: {line}"),
            TomlishError::UnterminatedArray(key) => write!(f, "unterminated TOML array for key `{key}`"),
            TomlishError::UnsupportedValue(value) => write!(f, "unsupported TOML value: {value}"),
            TomlishError::UnterminatedString(value) => write!(f, "unterminated TOML string: {value}"),
        }
    }
}

impl Error for TomlishError {}

#[cfg(test)]
mod tests {
    use super::{parse, TomlValue};

    #[test]
    fn parses_basic_document() {
        let doc = parse(
            r#"
            output_dir = "articles"
            stdout = true
            request_timeout_sec = 20
            [download]
            urls = ["https://a", "https://b"]
            "#,
        )
        .unwrap();

        assert_eq!(doc.get("output_dir"), Some(&TomlValue::String("articles".into())));
        assert_eq!(doc.get("stdout"), Some(&TomlValue::Bool(true)));
        assert_eq!(doc.get("request_timeout_sec"), Some(&TomlValue::Integer(20)));
        assert_eq!(
            doc.get("download.urls"),
            Some(&TomlValue::StringArray(vec!["https://a".into(), "https://b".into()]))
        );
    }
}
