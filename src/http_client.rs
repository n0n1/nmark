use std::error::Error;
use std::fmt;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, InvalidHeaderValue, ACCEPT, ACCEPT_LANGUAGE};
use reqwest::redirect::Policy;
use reqwest::Client;
use reqwest::StatusCode;

pub type HttpResult<T> = Result<T, HttpError>;

const DEFAULT_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
const DEFAULT_ACCEPT: &str =
    "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8";
const DEFAULT_ACCEPT_LANGUAGE: &str = "en-US,en;q=0.9,ru;q=0.8";
const DEFAULT_REQUEST_TIMEOUT_SEC: u64 = 20;
const DEFAULT_CONNECT_TIMEOUT_SEC: u64 = 10;
const DEFAULT_MAX_REDIRECTS: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpConfig {
    pub user_agent: String,
    pub accept: String,
    pub accept_language: String,
    pub request_timeout_sec: u64,
    pub connect_timeout_sec: u64,
    pub max_redirects: usize,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            user_agent: DEFAULT_USER_AGENT.to_string(),
            accept: DEFAULT_ACCEPT.to_string(),
            accept_language: DEFAULT_ACCEPT_LANGUAGE.to_string(),
            request_timeout_sec: DEFAULT_REQUEST_TIMEOUT_SEC,
            connect_timeout_sec: DEFAULT_CONNECT_TIMEOUT_SEC,
            max_redirects: DEFAULT_MAX_REDIRECTS,
        }
    }
}

#[derive(Debug)]
pub enum HttpError {
    Build(reqwest::Error),
    InvalidHeaderValue {
        header: &'static str,
        source: InvalidHeaderValue,
    },
    Request {
        url: String,
        source: reqwest::Error,
    },
    UnexpectedStatus {
        url: String,
        status: StatusCode,
    },
    ReadBody {
        url: String,
        source: reqwest::Error,
    },
}

pub fn build_client(config: &HttpConfig) -> HttpResult<Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_str(&config.accept).map_err(|source| HttpError::InvalidHeaderValue {
            header: "Accept",
            source,
        })?,
    );
    headers.insert(
        ACCEPT_LANGUAGE,
        HeaderValue::from_str(&config.accept_language).map_err(|source| {
            HttpError::InvalidHeaderValue {
                header: "Accept-Language",
                source,
            }
        })?,
    );

    Client::builder()
        .user_agent(config.user_agent.clone())
        .default_headers(headers)
        .connect_timeout(Duration::from_secs(config.connect_timeout_sec))
        .timeout(Duration::from_secs(config.request_timeout_sec))
        .redirect(Policy::limited(config.max_redirects))
        .build()
        .map_err(HttpError::Build)
}

pub async fn fetch_html(client: &Client, url: &str) -> HttpResult<String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|source| HttpError::Request {
            url: url.to_string(),
            source,
        })?;
    let final_url = response.url().to_string();
    let status = response.status();
    if !status.is_success() {
        return Err(HttpError::UnexpectedStatus {
            url: final_url,
            status,
        });
    }

    response
        .text()
        .await
        .map_err(|source| HttpError::ReadBody {
            url: final_url,
            source,
        })
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpError::Build(error) => write!(f, "failed to build http client: {error}"),
            HttpError::InvalidHeaderValue { header, source } => {
                write!(f, "invalid value for `{header}` header: {source}")
            }
            HttpError::Request { url, source } => write!(f, "request to `{url}` failed: {source}"),
            HttpError::UnexpectedStatus { url, status } => {
                write!(f, "server returned unexpected status {status} for `{url}`")
            }
            HttpError::ReadBody { url, source } => {
                write!(f, "failed to read response body from `{url}`: {source}")
            }
        }
    }
}

impl Error for HttpError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            HttpError::Build(error) => Some(error),
            HttpError::InvalidHeaderValue { source, .. } => Some(source),
            HttpError::Request { source, .. } => Some(source),
            HttpError::UnexpectedStatus { .. } => None,
            HttpError::ReadBody { source, .. } => Some(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{build_client, HttpConfig};

    #[test]
    fn builds_configured_client() {
        assert!(build_client(&HttpConfig::default()).is_ok());
    }

    #[test]
    fn rejects_invalid_accept_header() {
        let config = HttpConfig {
            accept: "\n".into(),
            ..HttpConfig::default()
        };
        assert!(build_client(&config).is_err());
    }
}
