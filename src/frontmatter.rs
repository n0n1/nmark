use std::error::Error;
use std::fmt;
use std::process::Command;
use std::process::ExitStatus;

pub type FrontmatterResult<T> = Result<T, FrontmatterError>;

#[derive(Debug)]
pub enum FrontmatterError {
    Io(std::io::Error),
    Utf8(std::string::FromUtf8Error),
    DateCommandFailed {
        command: &'static str,
        status: ExitStatus,
    },
    InvalidUtcOffset(String),
}

pub fn build_frontmatter(
    source: &str,
    author: Option<&str>,
    tags: &[String],
) -> FrontmatterResult<String> {
    let created = current_timestamp()?;
    let mut lines = vec!["---".to_string(), format!("created: {created}")];

    if !tags.is_empty() {
        let rendered_tags = tags
            .iter()
            .map(|tag| yaml_quote(tag))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("tags: [{rendered_tags}]"));
    }

    lines.push(format!("source: {}", yaml_quote(source)));

    if let Some(author) = author.filter(|value| !value.trim().is_empty()) {
        lines.push(format!("author: {}", yaml_quote(author)));
    }

    lines.push("---".to_string());
    Ok(lines.join("\n"))
}

fn current_timestamp() -> FrontmatterResult<String> {
    let timestamp = command_output("date", &["+%Y-%m-%dT%H:%M:%S"])?;
    let offset = command_output("date", &["+%z"])?;
    let formatted_offset = format_utc_offset(&offset)?;
    Ok(format!("{timestamp} (UTC {formatted_offset})"))
}

fn command_output(command: &'static str, args: &[&str]) -> FrontmatterResult<String> {
    let output = Command::new(command).args(args).output()?;
    if !output.status.success() {
        return Err(FrontmatterError::DateCommandFailed {
            command,
            status: output.status,
        });
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn format_utc_offset(offset: &str) -> FrontmatterResult<String> {
    if offset.len() != 5 {
        return Err(FrontmatterError::InvalidUtcOffset(offset.to_string()));
    }

    Ok(format!("{}:{}", &offset[..3], &offset[3..]))
}

fn yaml_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

impl fmt::Display for FrontmatterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrontmatterError::Io(error) => write!(f, "i/o failed: {error}"),
            FrontmatterError::Utf8(error) => write!(f, "utf-8 decoding failed: {error}"),
            FrontmatterError::DateCommandFailed { command, status } => {
                write!(f, "command `{command}` failed with status {status}")
            }
            FrontmatterError::InvalidUtcOffset(offset) => {
                write!(f, "unexpected UTC offset format: {offset}")
            }
        }
    }
}

impl Error for FrontmatterError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrontmatterError::Io(error) => Some(error),
            FrontmatterError::Utf8(error) => Some(error),
            FrontmatterError::DateCommandFailed { .. } => None,
            FrontmatterError::InvalidUtcOffset(_) => None,
        }
    }
}

impl From<std::io::Error> for FrontmatterError {
    fn from(value: std::io::Error) -> Self {
        FrontmatterError::Io(value)
    }
}

impl From<std::string::FromUtf8Error> for FrontmatterError {
    fn from(value: std::string::FromUtf8Error) -> Self {
        FrontmatterError::Utf8(value)
    }
}

#[cfg(test)]
mod tests {
    use super::{format_utc_offset, FrontmatterError};

    #[test]
    fn formats_utc_offset() {
        assert_eq!(format_utc_offset("+0300").unwrap(), "+03:00");
    }

    #[test]
    fn rejects_invalid_utc_offset() {
        let error = format_utc_offset("+03").unwrap_err();
        assert!(matches!(error, FrontmatterError::InvalidUtcOffset(_)));
    }
}
