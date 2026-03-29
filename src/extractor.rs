use std::error::Error;
use std::fmt;

use readabilityrs::{Article, Readability};

pub type ExtractorResult<T> = Result<T, ExtractorError>;

#[derive(Debug)]
pub enum ExtractorError {
    Readability(readabilityrs::ReadabilityError),
    MissingArticle,
}

pub fn extract_article(html: &str, url: &str) -> ExtractorResult<Article> {
    let readability = Readability::new(html, Some(url), None)
        .map_err(ExtractorError::Readability)?;
    let article = readability.parse().ok_or(ExtractorError::MissingArticle)?;
    Ok(article)
}

impl fmt::Display for ExtractorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExtractorError::Readability(error) => write!(f, "readability extraction failed: {error}"),
            ExtractorError::MissingArticle => f.write_str("article not found"),
        }
    }
}

impl Error for ExtractorError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ExtractorError::Readability(error) => Some(error),
            ExtractorError::MissingArticle => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_article, ExtractorError};

    #[test]
    fn rejects_missing_article() {
        let html = "<html><body><div>no article here</div></body></html>";
        let error = extract_article(html, "https://example.com").unwrap_err();
        assert!(matches!(error, ExtractorError::MissingArticle));
    }
}
