use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::tomlish::{self, TomlValue, TomlishError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputSource {
    Url(String),
    File(PathBuf),
}

pub type InputResult<T> = Result<T, InputError>;

#[derive(Debug)]
pub enum InputError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    UnsupportedFileFormat(PathBuf),
    EmptyInput(PathBuf),
    DownloadListNotFound(PathBuf),
    TomlParse {
        path: PathBuf,
        source: TomlishError,
    },
}

pub fn load_urls(source: &InputSource) -> InputResult<Vec<String>> {
    match source {
        InputSource::Url(url) => Ok(vec![url.clone()]),
        InputSource::File(path) => load_urls_from_file(path),
    }
}

pub fn looks_like_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

fn load_urls_from_file(path: &Path) -> InputResult<Vec<String>> {
    let content = fs::read_to_string(path).map_err(|source| InputError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    let extension = path.extension().and_then(|ext| ext.to_str());
    let urls = match extension {
        Some("txt") => parse_txt_urls(&content),
        Some("toml") => return parse_toml_urls(path, &content),
        _ => return Err(InputError::UnsupportedFileFormat(path.to_path_buf())),
    };

    if urls.is_empty() {
        return Err(InputError::EmptyInput(path.to_path_buf()));
    }

    Ok(urls)
}

fn parse_txt_urls(content: &str) -> Vec<String> {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_toml_urls(path: &Path, content: &str) -> InputResult<Vec<String>> {
    let doc = tomlish::parse(content).map_err(|source| InputError::TomlParse {
        path: path.to_path_buf(),
        source,
    })?;

    let mut urls = Vec::new();
    for key in ["download.urls", "urls"] {
        if let Some(value) = doc.get(key) {
            match value {
                TomlValue::String(_) => {}
                TomlValue::StringArray(items) => {
                    for item in items {
                        push_unique(&mut urls, item);
                    }
                }
                TomlValue::Bool(_) | TomlValue::Integer(_) => {}
            }
        }
    }

    if urls.is_empty() {
        return Err(InputError::DownloadListNotFound(path.to_path_buf()));
    }

    Ok(urls)
}

fn push_unique(urls: &mut Vec<String>, value: &str) {
    let value = value.trim();
    if value.is_empty() || urls.iter().any(|existing| existing == value) {
        return;
    }
    urls.push(value.to_string());
}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputError::Io { path, source } => {
                write!(f, "failed to read input file `{}`: {source}", path.display())
            }
            InputError::UnsupportedFileFormat(path) => write!(
                f,
                "unsupported input file format for `{}`; expected .txt or .toml",
                path.display()
            ),
            InputError::EmptyInput(path) => {
                write!(f, "input file `{}` does not contain any URLs", path.display())
            }
            InputError::DownloadListNotFound(path) => write!(
                f,
                "downloads list not found in `{}`; expected `urls = [\"...\"]` or `[download].urls = [\"...\"]`",
                path.display()
            ),
            InputError::TomlParse { path, source } => {
                write!(f, "failed to parse TOML input `{}`: {source}", path.display())
            }
        }
    }
}

impl Error for InputError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            InputError::Io { source, .. } => Some(source),
            InputError::UnsupportedFileFormat(_) => None,
            InputError::EmptyInput(_) => None,
            InputError::DownloadListNotFound(_) => None,
            InputError::TomlParse { source, .. } => Some(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{load_urls, InputError, InputSource};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn loads_txt_urls() {
        let path = temp_file("urls.txt");
        fs::write(&path, "# comment\nhttps://a\n\nhttps://b\n").unwrap();
        let urls = load_urls(&InputSource::File(path.clone())).unwrap();
        assert_eq!(urls, vec!["https://a", "https://b"]);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn loads_toml_urls() {
        let path = temp_file("download.toml");
        fs::write(
            &path,
            "urls = [\"https://a\"]\n[download]\nurls = [\"https://b\"]\n",
        )
        .unwrap();
        let mut urls = load_urls(&InputSource::File(path.clone())).unwrap();
        urls.sort();
        assert_eq!(urls, vec!["https://a", "https://b"]);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_single_url_toml_form() {
        let path = temp_file("download.toml");
        fs::write(&path, "url = \"https://a\"\n").unwrap();
        let error = load_urls(&InputSource::File(path.clone())).unwrap_err();
        assert!(matches!(error, InputError::DownloadListNotFound(found) if found == path));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_toml_without_download_list() {
        let path = temp_file("download.toml");
        fs::write(&path, "output_dir = \"articles\"\n").unwrap();
        let error = load_urls(&InputSource::File(path.clone())).unwrap_err();
        assert!(matches!(error, InputError::DownloadListNotFound(found) if found == path));
        let _ = fs::remove_file(path);
    }

    fn temp_file(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("rmark-{nonce}-{name}"))
    }
}
