use std::error::Error;
use std::fmt;

use crate::cli::CliError;
use crate::extractor::ExtractorError;
use crate::frontmatter::FrontmatterError;
use crate::http_client::HttpError;
use crate::settings::SettingsError;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
pub enum AppError {
    BatchFailed {
        failed: usize,
        total: usize,
    },
    Cli(CliError),
    Http(HttpError),
    Io(std::io::Error),
    Extractor(ExtractorError),
    Frontmatter(FrontmatterError),
    Settings(SettingsError),
    MissingContent,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::BatchFailed { failed, total } => {
                write!(f, "{failed} of {total} URLs failed")
            }
            AppError::Cli(error) => write!(f, "{error}"),
            AppError::Http(error) => write!(f, "http request failed: {error}"),
            AppError::Io(error) => write!(f, "i/o failed: {error}"),
            AppError::Extractor(error) => write!(f, "article extraction failed: {error}"),
            AppError::Frontmatter(error) => write!(f, "frontmatter generation failed: {error}"),
            AppError::Settings(error) => write!(f, "{error}"),
            AppError::MissingContent => f.write_str("empty content"),
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            AppError::BatchFailed { .. } => None,
            AppError::Cli(error) => Some(error),
            AppError::Http(error) => Some(error),
            AppError::Io(error) => Some(error),
            AppError::Extractor(error) => Some(error),
            AppError::Frontmatter(error) => Some(error),
            AppError::Settings(error) => Some(error),
            AppError::MissingContent => None,
        }
    }
}

impl From<CliError> for AppError {
    fn from(value: CliError) -> Self {
        AppError::Cli(value)
    }
}

impl From<HttpError> for AppError {
    fn from(value: HttpError) -> Self {
        AppError::Http(value)
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        AppError::Io(value)
    }
}

impl From<ExtractorError> for AppError {
    fn from(value: ExtractorError) -> Self {
        AppError::Extractor(value)
    }
}

impl From<FrontmatterError> for AppError {
    fn from(value: FrontmatterError) -> Self {
        AppError::Frontmatter(value)
    }
}

impl From<SettingsError> for AppError {
    fn from(value: SettingsError) -> Self {
        AppError::Settings(value)
    }
}
