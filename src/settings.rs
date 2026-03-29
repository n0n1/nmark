use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::{RunOptions, SourceArg};
use crate::http_client::HttpConfig;
use crate::inputs::{self, InputError, InputSource};
use crate::tomlish::{self, TomlValue, TomlishError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConfig {
    pub urls: Vec<String>,
    pub output_dir: PathBuf,
    pub write_to_stdout: bool,
    pub include_frontmatter: bool,
    pub http: HttpConfig,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct FileConfig {
    output_dir: Option<PathBuf>,
    write_to_stdout: Option<bool>,
    include_frontmatter: Option<bool>,
    http: HttpConfigOverrides,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct HttpConfigOverrides {
    user_agent: Option<String>,
    accept: Option<String>,
    accept_language: Option<String>,
    request_timeout_sec: Option<u64>,
    connect_timeout_sec: Option<u64>,
    max_redirects: Option<usize>,
}

pub type SettingsResult<T> = Result<T, SettingsError>;

#[derive(Debug)]
pub enum SettingsError {
    CurrentDir(std::io::Error),
    ConfigRead {
        path: PathBuf,
        source: std::io::Error,
    },
    ConfigParse {
        path: PathBuf,
        source: TomlishError,
    },
    InvalidConfigValue {
        path: PathBuf,
        key: &'static str,
        value: String,
        expected: &'static str,
    },
    Input(InputError),
    MissingInput,
}

pub fn resolve_config(options: RunOptions) -> SettingsResult<ResolvedConfig> {
    let cwd = env::current_dir().map_err(SettingsError::CurrentDir)?;
    let global = global_config_path();
    resolve_config_from(options, cwd, global)
}

fn resolve_config_from(
    options: RunOptions,
    cwd: PathBuf,
    global_config: Option<PathBuf>,
) -> SettingsResult<ResolvedConfig> {
    let mut merged = FileConfig::default();

    if let Some(path) = global_config.filter(|path| path.is_file()) {
        merged.merge(load_file_config(&path)?);
    }

    let local_config = cwd.join("config.toml");
    if local_config.is_file() {
        merged.merge(load_file_config(&local_config)?);
    }

    apply_cli_overrides(&mut merged, &options, &cwd);

    let source = match options.source {
        Some(SourceArg::Url(url)) => Some(InputSource::Url(url)),
        Some(SourceArg::Input(path)) => Some(InputSource::File(resolve_path(&cwd, &path))),
        None => None,
    };

    let source = if let Some(source) = source {
        source
    } else {
        let default_download = cwd.join("download.toml");
        if default_download.is_file() {
            InputSource::File(default_download)
        } else {
            return Err(SettingsError::MissingInput);
        }
    };
    let urls = inputs::load_urls(&source).map_err(SettingsError::Input)?;
    let output_dir = merged.output_dir.unwrap_or(cwd);

    Ok(ResolvedConfig {
        urls,
        output_dir,
        write_to_stdout: merged.write_to_stdout.unwrap_or(false),
        include_frontmatter: merged.include_frontmatter.unwrap_or(true),
        http: merged.http.resolve(),
    })
}

fn load_file_config(path: &Path) -> SettingsResult<FileConfig> {
    let content = fs::read_to_string(path).map_err(|source| SettingsError::ConfigRead {
        path: path.to_path_buf(),
        source,
    })?;
    let doc = tomlish::parse(&content).map_err(|source| SettingsError::ConfigParse {
        path: path.to_path_buf(),
        source,
    })?;
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));

    let output_dir = doc
        .get("output_dir")
        .and_then(toml_string)
        .filter(|value| !value.is_empty())
        .map(|value| resolve_path(base_dir, value));

    let write_to_stdout = doc
        .get("stdout")
        .and_then(toml_bool)
        .or_else(|| doc.get("write_to_stdout").and_then(toml_bool));

    let include_frontmatter = doc
        .get("include_frontmatter")
        .and_then(toml_bool)
        .or_else(|| doc.get("frontmatter").and_then(toml_bool));

    let http = HttpConfigOverrides {
        user_agent: doc
            .get("http.user_agent")
            .and_then(toml_string)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        accept: doc
            .get("http.accept")
            .and_then(toml_string)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        accept_language: doc
            .get("http.accept_language")
            .and_then(toml_string)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        request_timeout_sec: parse_nonzero_u64(
            path,
            &doc,
            "http.request_timeout_sec",
            "a positive integer number of seconds",
        )?,
        connect_timeout_sec: parse_nonzero_u64(
            path,
            &doc,
            "http.connect_timeout_sec",
            "a positive integer number of seconds",
        )?,
        max_redirects: parse_usize(path, &doc, "http.max_redirects", "a non-negative integer")?,
    };

    Ok(FileConfig {
        output_dir,
        write_to_stdout,
        include_frontmatter,
        http,
    })
}

fn apply_cli_overrides(config: &mut FileConfig, options: &RunOptions, cwd: &Path) {
    if let Some(output_dir) = &options.output_dir {
        config.output_dir = Some(resolve_path(cwd, output_dir));
    }
    if let Some(write_to_stdout) = options.write_to_stdout {
        config.write_to_stdout = Some(write_to_stdout);
    }
    if let Some(include_frontmatter) = options.include_frontmatter {
        config.include_frontmatter = Some(include_frontmatter);
    }
}

fn resolve_path(base_dir: &Path, path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn global_config_path() -> Option<PathBuf> {
    if let Ok(xdg_config_home) = env::var("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg_config_home).join("rmark").join("config.toml"));
    }

    env::var("HOME")
        .ok()
        .map(|home| PathBuf::from(home).join(".config").join("rmark").join("config.toml"))
}

fn toml_string(value: &TomlValue) -> Option<&str> {
    match value {
        TomlValue::String(value) => Some(value),
        _ => None,
    }
}

fn toml_bool(value: &TomlValue) -> Option<bool> {
    match value {
        TomlValue::Bool(value) => Some(*value),
        _ => None,
    }
}

fn parse_nonzero_u64(
    path: &Path,
    doc: &std::collections::HashMap<String, TomlValue>,
    key: &'static str,
    expected: &'static str,
) -> SettingsResult<Option<u64>> {
    let Some(value) = doc.get(key) else {
        return Ok(None);
    };

    let parsed = match value {
        TomlValue::Integer(value) => u64::try_from(*value).ok(),
        TomlValue::String(value) => value.parse().ok(),
        _ => None,
    }
    .filter(|value| *value > 0);

    parsed.ok_or_else(|| SettingsError::InvalidConfigValue {
        path: path.to_path_buf(),
        key,
        value: config_value_repr(value),
        expected,
    })
    .map(Some)
}

fn parse_usize(
    path: &Path,
    doc: &std::collections::HashMap<String, TomlValue>,
    key: &'static str,
    expected: &'static str,
) -> SettingsResult<Option<usize>> {
    let Some(value) = doc.get(key) else {
        return Ok(None);
    };

    let parsed = match value {
        TomlValue::Integer(value) => usize::try_from(*value).ok(),
        TomlValue::String(value) => value.parse().ok(),
        _ => None,
    };

    parsed.ok_or_else(|| SettingsError::InvalidConfigValue {
        path: path.to_path_buf(),
        key,
        value: config_value_repr(value),
        expected,
    })
    .map(Some)
}

fn config_value_repr(value: &TomlValue) -> String {
    match value {
        TomlValue::String(value) => format!("{value:?}"),
        TomlValue::Bool(value) => value.to_string(),
        TomlValue::Integer(value) => value.to_string(),
        TomlValue::StringArray(values) => format!("{values:?}"),
    }
}

impl FileConfig {
    fn merge(&mut self, other: FileConfig) {
        if other.output_dir.is_some() {
            self.output_dir = other.output_dir;
        }
        if other.write_to_stdout.is_some() {
            self.write_to_stdout = other.write_to_stdout;
        }
        if other.include_frontmatter.is_some() {
            self.include_frontmatter = other.include_frontmatter;
        }
        self.http.merge(other.http);
    }
}

impl HttpConfigOverrides {
    fn merge(&mut self, other: HttpConfigOverrides) {
        if other.user_agent.is_some() {
            self.user_agent = other.user_agent;
        }
        if other.accept.is_some() {
            self.accept = other.accept;
        }
        if other.accept_language.is_some() {
            self.accept_language = other.accept_language;
        }
        if other.request_timeout_sec.is_some() {
            self.request_timeout_sec = other.request_timeout_sec;
        }
        if other.connect_timeout_sec.is_some() {
            self.connect_timeout_sec = other.connect_timeout_sec;
        }
        if other.max_redirects.is_some() {
            self.max_redirects = other.max_redirects;
        }
    }

    fn resolve(self) -> HttpConfig {
        let defaults = HttpConfig::default();
        HttpConfig {
            user_agent: self.user_agent.unwrap_or(defaults.user_agent),
            accept: self.accept.unwrap_or(defaults.accept),
            accept_language: self.accept_language.unwrap_or(defaults.accept_language),
            request_timeout_sec: self
                .request_timeout_sec
                .unwrap_or(defaults.request_timeout_sec),
            connect_timeout_sec: self
                .connect_timeout_sec
                .unwrap_or(defaults.connect_timeout_sec),
            max_redirects: self.max_redirects.unwrap_or(defaults.max_redirects),
        }
    }
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettingsError::CurrentDir(source) => write!(f, "failed to read current directory: {source}"),
            SettingsError::ConfigRead { path, source } => {
                write!(f, "failed to read config `{}`: {source}", path.display())
            }
            SettingsError::ConfigParse { path, source } => {
                write!(f, "failed to parse config `{}`: {source}", path.display())
            }
            SettingsError::InvalidConfigValue {
                path,
                key,
                value,
                expected,
            } => write!(
                f,
                "invalid value for `{key}` in config `{}`: {value}; expected {expected}",
                path.display()
            ),
            SettingsError::Input(source) => write!(f, "{source}"),
            SettingsError::MissingInput => f.write_str(
                "no input source provided; pass a URL, use --input, or create `download.toml` in the current directory",
            ),
        }
    }
}

impl Error for SettingsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SettingsError::CurrentDir(source) => Some(source),
            SettingsError::ConfigRead { source, .. } => Some(source),
            SettingsError::ConfigParse { source, .. } => Some(source),
            SettingsError::InvalidConfigValue { .. } => None,
            SettingsError::Input(source) => Some(source),
            SettingsError::MissingInput => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{resolve_config_from, ResolvedConfig};
    use crate::cli::{RunOptions, SourceArg};
    use crate::http_client::HttpConfig;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn cli_overrides_local_and_global_config() {
        let root = temp_dir("config-merge");
        let global_dir = root.join("global");
        let cwd = root.join("cwd");
        fs::create_dir_all(global_dir.join("rmark")).unwrap();
        fs::create_dir_all(&cwd).unwrap();

        fs::write(
            global_dir.join("rmark").join("config.toml"),
            "output_dir = \"global-out\"\nstdout = false\n",
        )
        .unwrap();
        fs::write(
            cwd.join("config.toml"),
            "output_dir = \"local-out\"\nstdout = false\n",
        )
        .unwrap();
        fs::write(cwd.join("cli.txt"), "https://from-cli\n").unwrap();

        let resolved = resolve_config_from(
            RunOptions {
                source: Some(SourceArg::Input(PathBuf::from("cli.txt"))),
                output_dir: Some(PathBuf::from("cli-out")),
                write_to_stdout: Some(true),
                include_frontmatter: Some(false),
            },
            cwd.clone(),
            Some(global_dir.join("rmark").join("config.toml")),
        )
        .unwrap();

        assert_eq!(
            resolved,
            ResolvedConfig {
                urls: vec!["https://from-cli".into()],
                output_dir: cwd.join("cli-out"),
                write_to_stdout: true,
                include_frontmatter: false,
                http: HttpConfig::default(),
            }
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ignores_input_source_keys_in_config() {
        let cwd = temp_dir("config-ignores-source");
        fs::create_dir_all(&cwd).unwrap();
        fs::write(
            cwd.join("config.toml"),
            "input = \"download.txt\"\nurl = \"https://from-config\"\noutput_dir = \"articles\"\n",
        )
        .unwrap();

        let error = resolve_config_from(
            RunOptions {
                source: None,
                output_dir: None,
                write_to_stdout: None,
                include_frontmatter: None,
            },
            cwd.clone(),
            None,
        )
        .unwrap_err();

        assert!(matches!(error, super::SettingsError::MissingInput));

        let _ = fs::remove_dir_all(cwd);
    }

    #[test]
    fn falls_back_to_download_toml_in_cwd() {
        let cwd = temp_dir("download-default");
        fs::create_dir_all(&cwd).unwrap();
        fs::write(cwd.join("download.toml"), "urls = [\"https://a\"]\n").unwrap();

        let resolved = resolve_config_from(
            RunOptions {
                source: None,
                output_dir: None,
                write_to_stdout: None,
                include_frontmatter: None,
            },
            cwd.clone(),
            None,
        )
        .unwrap();

        assert_eq!(resolved.urls, vec!["https://a"]);
        assert_eq!(resolved.output_dir, cwd);
        assert_eq!(resolved.http, HttpConfig::default());

        let _ = fs::remove_dir_all(resolved.output_dir);
    }

    #[test]
    fn loads_http_config_from_settings_file() {
        let cwd = temp_dir("http-config");
        fs::create_dir_all(&cwd).unwrap();
        fs::write(
            cwd.join("config.toml"),
            concat!(
                "output_dir = \"articles\"\n",
                "[http]\n",
                "user_agent = \"nmark-test/1.0\"\n",
                "connect_timeout_sec = 3\n",
                "request_timeout_sec = 7\n",
                "max_redirects = 2\n",
                "accept_language = \"ru,en;q=0.8\"\n",
            ),
        )
        .unwrap();
        fs::write(cwd.join("download.toml"), "urls = [\"https://a\"]\n").unwrap();

        let resolved = resolve_config_from(
            RunOptions {
                source: None,
                output_dir: None,
                write_to_stdout: None,
                include_frontmatter: None,
            },
            cwd.clone(),
            None,
        )
        .unwrap();

        assert_eq!(resolved.http.user_agent, "nmark-test/1.0");
        assert_eq!(resolved.http.connect_timeout_sec, 3);
        assert_eq!(resolved.http.request_timeout_sec, 7);
        assert_eq!(resolved.http.max_redirects, 2);
        assert_eq!(resolved.http.accept_language, "ru,en;q=0.8");

        let _ = fs::remove_dir_all(cwd);
    }

    #[test]
    fn rejects_invalid_http_timeout_config() {
        let cwd = temp_dir("invalid-http-config");
        fs::create_dir_all(&cwd).unwrap();
        fs::write(
            cwd.join("config.toml"),
            "[http]\nrequest_timeout_sec = \"fast\"\n",
        )
        .unwrap();
        fs::write(cwd.join("download.toml"), "urls = [\"https://a\"]\n").unwrap();

        let error = resolve_config_from(
            RunOptions {
                source: None,
                output_dir: None,
                write_to_stdout: None,
                include_frontmatter: None,
            },
            cwd.clone(),
            None,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            super::SettingsError::InvalidConfigValue {
                key: "http.request_timeout_sec",
                ..
            }
        ));

        let _ = fs::remove_dir_all(cwd);
    }

    fn temp_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("rmark-{nonce}-{name}"))
    }
}
