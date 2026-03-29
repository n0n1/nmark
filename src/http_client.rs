use std::error::Error;
use std::fmt;
use std::time::Duration;

use reqwest::header::{
    HeaderMap, HeaderName, HeaderValue, InvalidHeaderValue, ACCEPT, ACCEPT_LANGUAGE, REFERER,
};
use reqwest::redirect::Policy;
use reqwest::{Client, RequestBuilder, Url};
use reqwest::StatusCode;

pub type HttpResult<T> = Result<T, HttpError>;

const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36";
const DEFAULT_ACCEPT: &str =
    "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8";
const DEFAULT_ACCEPT_LANGUAGE: &str = "en-US,en;q=0.9,ru;q=0.8";
const DEFAULT_REQUEST_TIMEOUT_SEC: u64 = 20;
const DEFAULT_CONNECT_TIMEOUT_SEC: u64 = 10;
const DEFAULT_MAX_REDIRECTS: usize = 10;
const UPGRADE_INSECURE_REQUESTS: HeaderName = HeaderName::from_static("upgrade-insecure-requests");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpConfig {
    pub user_agent: String,
    pub accept: String,
    pub accept_language: String,
    pub referer: Option<String>,
    pub upgrade_insecure_requests: bool,
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
            referer: None,
            upgrade_insecure_requests: true,
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
    if config.upgrade_insecure_requests {
        headers.insert(UPGRADE_INSECURE_REQUESTS, HeaderValue::from_static("1"));
    }

    Client::builder()
        .user_agent(config.user_agent.clone())
        .default_headers(headers)
        .connect_timeout(Duration::from_secs(config.connect_timeout_sec))
        .timeout(Duration::from_secs(config.request_timeout_sec))
        .redirect(Policy::limited(config.max_redirects))
        .build()
        .map_err(HttpError::Build)
}

pub async fn fetch_html(client: &Client, config: &HttpConfig, url: &str) -> HttpResult<String> {
    let response = apply_browser_headers(client.get(url), config, url)?
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

fn apply_browser_headers(
    request: RequestBuilder,
    config: &HttpConfig,
    url: &str,
) -> HttpResult<RequestBuilder> {
    let referer = config
        .referer
        .as_deref()
        .map(ToOwned::to_owned)
        .or_else(|| derive_referer(url));

    if let Some(referer) = referer {
        let value =
            HeaderValue::from_str(&referer).map_err(|source| HttpError::InvalidHeaderValue {
                header: "Referer",
                source,
            })?;
        Ok(request.header(REFERER, value))
    } else {
        Ok(request)
    }
}

fn derive_referer(url: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    let host = parsed.host_str()?;
    let mut referer = format!("{}://{}", parsed.scheme(), host);
    if let Some(port) = parsed.port() {
        referer.push(':');
        referer.push_str(&port.to_string());
    }
    referer.push('/');
    Some(referer)
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
    use super::{build_client, derive_referer, HttpConfig};

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

    #[test]
    fn derives_same_origin_referer() {
        assert_eq!(
            derive_referer("https://blog.dart.dev/path/to/article"),
            Some("https://blog.dart.dev/".into())
        );
    }
}
